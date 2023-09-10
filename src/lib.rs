#[macro_use]
extern crate log;
extern crate actix_identity;
extern crate actix_web;
extern crate envconfig;
extern crate pretty_env_logger;
extern crate reqwest;
#[macro_use]
extern crate serde_derive;
extern crate serde;
extern crate serde_json;
#[macro_use]
extern crate rusqlite;
#[macro_use]
extern crate lazy_static;

use crate::data::push::PushBuilder;
use envconfig::Envconfig;
use std::path::PathBuf;
use url::Url;

mod data;
mod external;
mod handlers;
mod poll;
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

    #[envconfig(
        from = "SESSION_SECRET",
        default = "abcdefghijklmnopqrstuvwxyz012345abcdefghijklmnopqrstuvwxyz012345"
    )]
    pub session_secret: String,

    #[envconfig(from = "VAPID_PUBLIC_KEY")]
    pub vapid_public_key: String,

    #[envconfig(from = "VAPID_PRIVATE_KEY")]
    pub vapid_private_key: String,

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

#[derive(Clone)]
pub struct AppState {
    config: Config,
    db: data::DbConn,
    push: data::push::PushBuilder,
}

pub fn init_env() -> anyhow::Result<AppState> {
    if std::env::var_os("RUST_LOG").is_none() {
        std::env::set_var("RUST_LOG", "info,kachiclash=debug");
    }
    //std::env::set_var("RUST_LOG", "debug");
    pretty_env_logger::init();

    let config = Config::init_from_env().expect("Could not read config from environment");
    if config.env != "dev"
        && config.session_secret
            == "abcdefghijklmnopqrstuvwxyz012345abcdefghijklmnopqrstuvwxyz012345"
    {
        panic!("default session_secret specified for non-dev deployment");
    }

    let db = data::make_conn(&config.db_path);
    let push = PushBuilder::with_base64_private_key(&config.vapid_private_key)?;

    Ok(AppState { config, db, push })
}

pub async fn run_server(app_state: &AppState) -> anyhow::Result<()> {
    server::run(app_state).await
}

pub async fn start_poll(app_state: &AppState) -> anyhow::Result<()> {
    poll::start(app_state).await
}
