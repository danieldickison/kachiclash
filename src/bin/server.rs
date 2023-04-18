extern crate kachiclash;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let app_state = kachiclash::init_env()?;
    kachiclash::start_poll(&app_state)?;
    kachiclash::run_server(&app_state).await
}
