use crate::AppState;
use tokio::time::interval;
use tokio::time::Duration;

pub async fn daily_results(_app_state: AppState) {
    let mut interval = interval(Duration::from_secs(3600));
    loop {
        interval.tick().await;
        // TODO: do work
        trace!("todo: daily_results poll tick");
    }
}
