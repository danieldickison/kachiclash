use crate::data::basho::update_torikumi;
use crate::data::BashoId;
use crate::data::RankDivision;
use crate::external::sumo_api;
use crate::AppState;
use tokio::time::interval;
use tokio::time::Duration;

const INTERVAL_DEV: u64 = 600;
const INTERVAL_PROD: u64 = 3600;

pub async fn daily_results(app_state: AppState) -> ! {
    let mut interval = interval(Duration::from_secs(if app_state.config.is_dev() {
        INTERVAL_DEV
    } else {
        INTERVAL_PROD
    }));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        match do_tick(&app_state).await {
            Ok(_) => trace!("do_tick succeeded"),
            Err(e) => error!("do_tick failed: {}", e),
        }
    }
}

async fn do_tick(app_state: &AppState) -> anyhow::Result<()> {
    trace!("daily_results tick starting");
    let basho_id: BashoId;
    let last_day: u8;
    {
        // Find the last completed day of the currently active basho. The current basho should be the only one in the db without any associated awards.
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
    trace!("querying sumo-api for basho {} day {}", basho_id.id(), day);
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
    } else {
        debug!("day {day} is not yet complete; sleeping");
    }

    Ok(())
}
