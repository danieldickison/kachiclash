extern crate kachiclash;

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    kachiclash::run_server().await
}
