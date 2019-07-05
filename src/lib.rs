#[macro_use]
extern crate log;
extern crate env_logger;
#[macro_use]
extern crate envconfig_derive;
extern crate envconfig;
extern crate actix_web;
extern crate failure;
extern crate reqwest;
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate rusqlite;

use std::path::PathBuf;
use envconfig::Envconfig;

mod data;
mod handlers;
mod server;


#[derive(Envconfig)]
#[derive(Clone)]
pub struct Config {
    #[envconfig(from = "KACHI_ENV", default = "dev")]
    pub env: String,

    #[envconfig(from = "KACHI_DB_PATH", default = "var/kachiclash.sqlite")]
    pub db_path: PathBuf,

    #[envconfig(from = "SESSION_SECRET", default = "abcdefghijklmnopqrstuvwxyz012345")]
    pub session_secret: String,
}

#[derive(Debug)]
pub struct AppState {
    db: data::DbConn,
}

pub fn run_server() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,kachiclash=debug");
    //std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let config = Config::init().expect("Could not read config from environment");
    if config.env != "dev" && config.session_secret == "abcdefghijklmnopqrstuvwxyz012345" {
        panic!("default session_secret specified for non-dev deployment");
    }
    
    server::run(config)
}
