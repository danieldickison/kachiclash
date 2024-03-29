use crate::data::BashoInfo;
use crate::AppState;
use tokio::time::interval;
use tokio::time::Duration;

pub async fn basho_prelude(app_state: AppState) {
    let mut interval = interval(Duration::from_secs(3600));
    interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        interval.tick().await;
        match do_tick(&app_state) {
            Ok(_) => info!("basho_prelude do_tick succeeded"),
            Err(e) => error!("basho_prelude do_tick failed: {}", e),
        }
    }
}

fn do_tick(app_state: &AppState) -> anyhow::Result<()> {
    trace!("basho_prelude do_tick");
    match BashoInfo::current_and_previous(&app_state.db.lock().unwrap())? {
        (Some(basho), _) => {
            if basho.has_started() {
                trace!("current basho {} is underway; no alerts", basho.id);
            } else {
                trace!("todo: maybe notify basho alert for {}", basho.id);
            }
        }
        _ => trace!("no current basho to alert"),
    }
    Ok(())
}
