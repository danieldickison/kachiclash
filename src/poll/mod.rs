use crate::AppState;
use tokio::task::spawn;

pub mod basho_prelude;
use basho_prelude::basho_prelude;

pub mod daily_results;
use daily_results::daily_results;

pub fn start(app_state: &AppState) -> anyhow::Result<()> {
    spawn(basho_prelude(app_state.clone()));
    spawn(daily_results(app_state.clone()));
    Ok(())
}
