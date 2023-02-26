#[macro_use]
extern crate log;
extern crate actix_identity;
extern crate actix_web;
extern crate env_logger;
extern crate envconfig;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate rusqlite;
#[macro_use]
extern crate lazy_static;

use envconfig::Envconfig;
use std::path::PathBuf;
use url::Url;

mod data;
mod external;
mod handlers;
mod server;
mod util;

#[derive(Envconfig, Clone, Debug)]
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

    #[envconfig(
        from = "KACHI_HERO",
        default = "/static/img2/2021-Kachi-Clash-Banner-2.png"
    )]
    pub hero_img_src: String,

    #[envconfig(from = "SESSION_SECRET", default = "abcdefghijklmnopqrstuvwxyz012345")]
    pub session_secret: String,

    #[envconfig(from = "PUSH_KEY_FILE", default = "var/dev.pem")]
    pub push_key_file: String,

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

// #[derive(Debug)]
pub struct AppState {
    config: Config,
    db: data::DbConn,
    push: data::push::PushBuilder,
}

pub async fn run_server() -> anyhow::Result<()> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info,kachiclash=debug");
    }
    //std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let config = Config::init_from_env().expect("Could not read config from environment");
    if config.env != "dev" && config.session_secret == "abcdefghijklmnopqrstuvwxyz012345" {
        panic!("default session_secret specified for non-dev deployment");
    }

    server::run(config).await
}
