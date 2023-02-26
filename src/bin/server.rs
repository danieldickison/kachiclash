extern crate kachiclash;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    kachiclash::run_server().await
}
