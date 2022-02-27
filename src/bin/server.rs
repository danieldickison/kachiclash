extern crate kachiclash;

#[tokio::main]
async fn main() -> std::io::Result<()> {
    kachiclash::run_server().await
}
