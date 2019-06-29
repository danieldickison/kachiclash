extern crate env_logger;
#[macro_use]
extern crate envconfig_derive;
extern crate envconfig;
#[macro_use]
extern crate slog;
extern crate actix_web;
extern crate failure;
extern crate reqwest;
extern crate serde_derive;
extern crate serde;
extern crate serde_json;

use std::path::PathBuf;
use std::convert::TryInto;
use actix_web::{http, server, App};
use actix_web::middleware::Logger;
use actix_web::middleware::session::{SessionStorage, CookieSessionBackend};
use envconfig::Envconfig;

mod data;
//mod external;
mod handlers;
mod logging;


#[derive(Envconfig)]
#[derive(Clone)]
pub struct Config {
    #[envconfig(from = "KACHI_ENV", default = "dev")]
    pub env: String,

    #[envconfig(from = "KACHI_DB_PATH", default = "kachi.db")]
    pub db_path: PathBuf,

    #[envconfig(from = "SESSION_SECRET", default = "abcdefghijklmnopqrstuvwxyz012345")]
    pub session_secret: String,
}

#[derive(Debug)]
pub struct AppState {
    log: slog::Logger,
    db: data::DbConn,
}

fn main() {
    let log = logging::setup_logging();

    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let config = Config::init().expect("Could not read config from environment");
    if config.env != "dev" && config.session_secret == "abcdefghijklmnopqrstuvwxyz012345" {
        panic!("default session_secret specified for non-dev deployment");
    }
    let session_secret: [u8; 32] = config.session_secret.as_bytes().try_into().expect("session key should be 32 utf8 bytes");
    
    info!(log, "starting server on localhost:8000");
    server::new(move || {
        App::with_state(AppState {
            log: log.clone(),
            db: data::schema::init_database(&log, &config.db_path),
        })
        .middleware(Logger::default())
        .middleware(
            SessionStorage::new(CookieSessionBackend::signed(&session_secret).secure(config.env != "dev"))
        )
        .resource("/", |r| {
            r.get().f(handlers::index)
        })
        .scope("/db", |db_scope|{
            db_scope.nested("/player", |player_scope| {
                player_scope
                    .resource("", |r| {
                        r.method(http::Method::GET).f(handlers::list_players)
                    })
            })
        })
        // .scope("/rest/v1", |v1_scope| {
        //     v1_scope.nested("/activities", |activities_scope| {
        //         activities_scope
        //             .resource("", |r| {
        //                 r.method(http::Method::GET).f(handlers::get_activities);
        //                 r.method(http::Method::POST)
        //                     .with_config(handlers::create_activity, |cfg| {
        //                         (cfg.0).1.error_handler(handlers::json_error_handler);
        //                     })
        //             })
        //             .resource("/{activity_id}", |r| {
        //                 r.method(http::Method::GET).with(handlers::get_activity);
        //                 r.method(http::Method::DELETE)
        //                     .with(handlers::delete_activity);
        //                 r.method(http::Method::PATCH)
        //                     .with_config(handlers::edit_activity, |cfg| {
        //                         (cfg.0).1.error_handler(handlers::json_error_handler);
        //                     });
        //             })
        //     })
        // })
        .resource("/health", |r| {
            r.method(http::Method::GET).f(handlers::health)
        })
        .resource("/name", |r| {
            r.method(http::Method::GET).f(handlers::name)
        })
        .finish()
    })
    .bind("0.0.0.0:8000")
    .unwrap()
    .run();
}
