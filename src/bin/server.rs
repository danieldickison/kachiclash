extern crate kachiclash;

use tokio::try_join;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = kachiclash::init_env()?;
    try_join!(kachiclash::run_server(&app_state)).map(|_| ())
}
