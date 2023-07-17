use crate::AppState;
use tokio::task::spawn;

pub mod basho_alert;
use basho_alert::basho_alert;

pub mod daily_results;
use daily_results::daily_results;

pub fn start(app_state: &AppState) -> anyhow::Result<()> {
    spawn(basho_alert(app_state.clone()));
    spawn(daily_results(app_state.clone()));
    Ok(())
}
