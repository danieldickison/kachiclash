use crate::AppState;
use tokio::{task::spawn, try_join};

pub mod basho_prelude;
use basho_prelude::basho_prelude;

pub mod daily_results;
use daily_results::daily_results;

pub async fn start(app_state: &AppState) -> anyhow::Result<()> {
    match try_join!(
        spawn(basho_prelude(app_state.clone())),
        spawn(daily_results(app_state.clone()))
    ) {
        Ok(_) => panic!("poll functions should never exit"),
        Err(e) => Err(e.into()),
    }
}
