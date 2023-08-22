use crate::data::basho::update_torikumi;
use crate::data::push::mass_notify_day_result;
use crate::data::BashoId;
use crate::data::RankDivision;
use crate::external::sumo_api;
use crate::AppState;
use chrono::Utc;
use chrono::{DateTime, FixedOffset, NaiveTime};
use tokio::time::sleep;

const POLL_INTERVAL_FAST: i64 = 300; // 5 min
const POLL_INTERVAL_SLOW: i64 = 3600; // 1 hour
const POLL_FAST_DURATION: i64 = 3600; // 1 hour
const ERROR_RETRY_WAIT: i64 = 15;
const ERROR_RETRY_MULTIPLIER: i32 = 2;

lazy_static! {
    static ref JST: FixedOffset = FixedOffset::east_opt(9 * 60 * 60).unwrap();
    static ref POLL_START_TIME_JST: NaiveTime = NaiveTime::from_hms_opt(18, 0, 0).unwrap();
}

pub async fn daily_results(app_state: AppState) -> ! {
    let mut retry_wait = chrono::Duration::seconds(ERROR_RETRY_WAIT);
    loop {
        let next_poll_datetime = match do_tick(&app_state).await {
            Ok(date) => {
                retry_wait = chrono::Duration::seconds(ERROR_RETRY_WAIT);
                date
            }
            Err(e) => {
                error!("do_tick error: {}", e);
                retry_wait = retry_wait * ERROR_RETRY_MULTIPLIER; // exponential backoff
                Utc::now() + retry_wait
            }
        };

        let sleep_dur = next_poll_datetime.signed_duration_since(Utc::now());
        debug!(
            "sleeping for {} until next poll at {}",
            PrettyDuration::from(sleep_dur),
            next_poll_datetime
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
impl From<chrono::Duration> for PrettyDuration {
    fn from(value: chrono::Duration) -> Self {
        Self(value)
    }
}

/// Checks sumo-api for next day's results. Returns the time at which we should poll.
async fn do_tick(app_state: &AppState) -> anyhow::Result<DateTime<Utc>> {
    trace!("daily_results tick starting");
    let basho_id: BashoId;
    let basho_start_date: DateTime<Utc>;
    let last_day: u8;
    {
        // Find the last completed day of the currently active basho. The current basho should be the only one in the db with picks but without any associated awards.
        let db = app_state.db.lock().unwrap();
        (basho_id, basho_start_date, last_day) = db.query_row(
            "
            SELECT
                basho.id,
                basho.start_date,
                COALESCE((SELECT MAX(day) FROM torikumi WHERE torikumi.basho_id = basho.id), 0)
            FROM basho
            WHERE
                NOT EXISTS (SELECT 1 FROM award WHERE award.basho_id = basho.id)
                AND EXISTS (SELECT 1 FROM pick WHERE pick.basho_id = basho.id)
            ", // This will error if multiple rows are returned
            (),
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
    }

    let day = last_day + 1;
    if day > 15 {
        warn!("Basho {:#} not finalized after day 15", basho_id);
        return Ok(Utc::now() + chrono::Duration::days(1));
    }

    let dry_run = matches!(std::env::var("SUMO_API_DRY_RUN"), Ok(val) if val == "1");
    if query_and_update(basho_id, day, app_state, dry_run).await? {
        if !dry_run {
            mass_notify_day_result(
                &app_state.db,
                &app_state.push,
                &app_state.config.url(),
                basho_id,
                day,
            )
            .await?;
        }
        Ok(next_poll_date(basho_start_date, day + 1))
    } else {
        debug!("Day {} results incomplete; going to sleep", day);
        Ok(next_poll_date(basho_start_date, day))
    }
}

pub async fn query_and_update(
    basho_id: BashoId,
    day: u8,
    app_state: &AppState,
    dry_run: bool,
) -> anyhow::Result<bool> {
    debug!("Querying sumo-api for basho {} day {}", basho_id.id(), day);
    let resp = sumo_api::BanzukeResponse::fetch(basho_id, RankDivision::Makuuchi).await?;
    let complete = resp.day_complete(day);
    if complete {
        let update_data = resp.torikumi_update_data(day);
        info!(
            "Got complete day {} results; updating db with {} bouts",
            day,
            update_data.len()
        );

        if dry_run {
            debug!("dry run; not updating db or sending push notifications");
        } else {
            let mut db = app_state.db.lock().unwrap();
            update_torikumi(&mut db, basho_id, day, &update_data)?;
        }
    }
    Ok(complete)
}

fn next_poll_date(basho_start_date: DateTime<Utc>, day: u8) -> DateTime<Utc> {
    let poll_start = basho_start_date
        .with_timezone(&*JST)
        .date_naive()
        .checked_add_days(chrono::Days::new(day as u64 - 1))
        .unwrap()
        .and_time(*POLL_START_TIME_JST)
        .and_local_timezone(*JST)
        .unwrap();

    let now = Utc::now().with_timezone(&*JST);
    if now < poll_start {
        poll_start.with_timezone(&Utc)
    } else if now < poll_start + chrono::Duration::seconds(POLL_FAST_DURATION) {
        now.with_timezone(&Utc) + chrono::Duration::seconds(POLL_INTERVAL_FAST)
    } else {
        now.with_timezone(&Utc) + chrono::Duration::seconds(POLL_INTERVAL_SLOW)
    }
}
