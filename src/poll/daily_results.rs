use std::time::Duration;

use crate::data::basho::update_torikumi;
use crate::data::push::mass_notify_day_result;
use crate::data::BashoId;
use crate::data::RankDivision;
use crate::external::sumo_api;
use crate::AppState;
use chrono::{DurationRound, Utc};
use chrono::{FixedOffset, NaiveTime};
use tokio::time::{interval, sleep};

const POLL_INTERVAL: u64 = 300; // 5 min
const POLL_DURATION: i64 = 3600; // 1 hour

pub async fn daily_results(app_state: AppState) -> ! {
    lazy_static! {
        static ref JST: FixedOffset = FixedOffset::east_opt(9 * 60 * 60).unwrap();
        static ref POLL_START_TIME: NaiveTime = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
        static ref POLL_END_TIME: NaiveTime = NaiveTime::from_hms_opt(20, 0, 0).unwrap();
    }
    let mut poll = interval(Duration::from_secs(POLL_INTERVAL));
    poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    loop {
        let now_jst = Utc::now().with_timezone(&*JST);
        let today_poll_start = now_jst
            .date_naive()
            .and_time(*POLL_START_TIME)
            .and_local_timezone(*JST)
            .unwrap();
        let today_poll_end = today_poll_start + chrono::Duration::seconds(POLL_DURATION);

        // sleep until 6pm JST when final results might be ready
        if now_jst < today_poll_start {
            let sleep_dur = today_poll_start.signed_duration_since(now_jst);
            debug!(
                "sleeping for {} until {}",
                PrettyDuration(sleep_dur),
                today_poll_start
            );
            sleep(sleep_dur.to_std().unwrap()).await;
        }

        // poll until 7pm JST by which time final results should've arrived
        let mut done = false;
        while !done && Utc::now() < today_poll_end {
            poll.tick().await;
            match do_tick(&app_state).await {
                Ok(true) => done = true,
                Ok(false) => debug!(
                    "final results have not arrived yet; sleeping for {:?}",
                    poll.period()
                ),
                Err(e) => error!("do_tick failed: {}", e),
            }
        }

        if !done {
            error!(
                "Failed to get final results from sumo-api before timeout {}",
                today_poll_end
            );
        }

        // sleep until midnight JST
        let midnight = (today_poll_start + chrono::Days::new(1))
            .duration_trunc(chrono::Duration::days(1))
            .unwrap();
        let sleep_dur = midnight.signed_duration_since(Utc::now());
        debug!(
            "sleeping for {:.3} until midnight {}",
            PrettyDuration(sleep_dur),
            midnight
        );
        sleep(sleep_dur.to_std().unwrap()).await;
    }
}

struct PrettyDuration(chrono::Duration);
impl std::fmt::Display for PrettyDuration {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut secs = self.0.num_milliseconds();
        if secs > 86_400_000 {
            f.write_fmt(format_args!("{}d", secs / 86_400_000))?;
            secs %= 86_400_000;
        }
        if secs > 3_600_000 {
            f.write_fmt(format_args!("{}h", secs / 3_600_000))?;
            secs %= 3_600_000;
        }
        if secs > 60_000 {
            f.write_fmt(format_args!("{}m", secs / 60_000))?;
            secs %= 60_000;
        }
        f.write_fmt(format_args!("{:.3}s", secs as f32 / 1000.0))
    }
}

async fn do_tick(app_state: &AppState) -> anyhow::Result<bool> {
    trace!("daily_results tick starting");
    let basho_id: BashoId;
    let last_day: u8;
    {
        // Find the last completed day of the currently active basho. The current basho should be the only one in the db with picks but without any associated awards.
        let db = app_state.db.lock().unwrap();
        (basho_id, last_day) = db.query_row(
            "
            SELECT
                basho.id,
                COALESCE((SELECT MAX(day) FROM torikumi WHERE torikumi.basho_id = basho.id), 0)
            FROM basho
            WHERE
                NOT EXISTS (SELECT 1 FROM award WHERE award.basho_id = basho.id)
                AND EXISTS (SELECT 1 FROM pick WHERE pick.basho_id = basho.id)
            ", // This will error if multiple rows are returned
            (),
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
    }

    let day = last_day + 1;
    debug!("querying sumo-api for basho {} day {}", basho_id.id(), day);
    let resp = sumo_api::BanzukeResponse::fetch(basho_id, RankDivision::Makuuchi).await?;
    if resp.day_complete(day) {
        let update_data = resp.torikumi_update_data(day);
        info!(
            "got complete day {} results; updating db with {} bouts",
            day,
            update_data.len()
        );
        {
            let mut db = app_state.db.lock().unwrap();
            update_torikumi(&mut db, basho_id, day, &update_data)?;
        }

        mass_notify_day_result(
            &app_state.db,
            &app_state.push,
            &app_state.config.url(),
            basho_id,
            day,
        )
        .await?;
        Ok(true)
    } else {
        Ok(false)
    }
}
