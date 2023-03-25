use crate::AppState;
use tokio::task::spawn;

mod basho_alert;
use basho_alert::basho_alert;

mod daily_results;
use daily_results::daily_results;

pub fn start(app_state: AppState) -> anyhow::Result<()> {
    spawn(basho_alert(app_state.clone()));
    spawn(daily_results(app_state.clone()));
    Ok(())
}
