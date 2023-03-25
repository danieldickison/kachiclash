use crate::AppState;
use tokio::time::Duration;
use tokio::time::interval;

pub async fn daily_results(app_state: AppState) {
    let mut interval = interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        // TODO: do work
        trace!("daily_results poll tick");
    }
}
