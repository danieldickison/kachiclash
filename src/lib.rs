#[macro_use]
extern crate log;
extern crate env_logger;
extern crate envconfig;
extern crate actix_web;
extern crate actix_identity;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate rusqlite;
#[macro_use]
extern crate lazy_static;

use url::Url;
use std::path::PathBuf;
use envconfig::Envconfig;

mod data;
mod handlers;
mod server;
mod external;


#[derive(Envconfig)]
#[derive(Clone, Debug)]
pub struct Config {
    #[envconfig(from = "KACHI_ENV", default = "dev")]
    pub env: String,

    #[envconfig(from = "KACHI_STATIC_PATH", default = "public")]
    pub static_path: PathBuf,

    #[envconfig(from = "KACHI_DB_PATH", default = "var/kachiclash.sqlite")]
    pub db_path: PathBuf,

    #[envconfig(from = "KACHI_HOST", default = "kachiclash.com")]
    pub host: String,

    #[envconfig(from = "KACHI_PORT")]
    pub port: u16,

    #[envconfig(from = "SESSION_SECRET", default = "abcdefghijklmnopqrstuvwxyz012345")]
    pub session_secret: String,

    #[envconfig(from = "DISCORD_CLIENT_ID")]
    pub discord_client_id: String,

    #[envconfig(from = "DISCORD_CLIENT_SECRET")]
    pub discord_client_secret: String,

    #[envconfig(from = "GOOGLE_CLIENT_ID")]
    pub google_client_id: String,

    #[envconfig(from = "GOOGLE_CLIENT_SECRET")]
    pub google_client_secret: String,

    #[envconfig(from = "REDDIT_CLIENT_ID")]
    pub reddit_client_id: String,

    #[envconfig(from = "REDDIT_CLIENT_SECRET")]
    pub reddit_client_secret: String,
}

impl Config {
    pub fn is_dev(&self) -> bool {
        self.env == "dev"
    }

    pub fn url(&self) -> Url {
        let mut url = Url::parse(format!("https://{}:{}", self.host, self.port).as_str())
            .expect("create base url for host");
        if self.is_dev() {
            url.set_scheme("http").expect("set scheme to unsecure http");
        } else {
            url.set_port(None).expect("remove url port");
        }
        url
    }
}

#[derive(Debug)]
pub struct AppState {
    config: Config,
    db: data::DbConn,
}

pub async fn run_server() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "info,kachiclash=debug");
    //std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let config = Config::init_from_env().expect("Could not read config from environment");
    if config.env != "dev" && config.session_secret == "abcdefghijklmnopqrstuvwxyz012345" {
        panic!("default session_secret specified for non-dev deployment");
    }
    
    server::run(config).await
}
