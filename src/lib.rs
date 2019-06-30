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

use std::path::PathBuf;
use std::convert::TryInto;
use actix_web::{web, HttpServer, App, HttpResponse};
use actix_web::middleware::Logger;
use actix_session::{CookieSession};
use envconfig::Envconfig;

mod data;
mod handlers;


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
    std::env::set_var("RUST_LOG", "actix_web=info");
    env_logger::init();

    let config = Config::init().expect("Could not read config from environment");
    if config.env != "dev" && config.session_secret == "abcdefghijklmnopqrstuvwxyz012345" {
        panic!("default session_secret specified for non-dev deployment");
    }
    let session_secret: [u8; 32] = config.session_secret.as_bytes().try_into().expect("session key should be 32 utf8 bytes");
    
    info!("starting server on localhost:8000");
    HttpServer::new(move || App::new()
        .data(AppState {
            db: data::make_conn(&config.db_path),
        })
        .wrap(Logger::default())
        .wrap(CookieSession::signed(&session_secret).secure(config.env != "dev"))
        .service(
            web::resource("/").to(handlers::index)
        )
        .service(
            web::scope("/db")
                .service(web::resource("/player").to(handlers::list_players))
        )
        .default_service(
            web::route().to(|| HttpResponse::NotFound())
        )
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
    )
    .bind("0.0.0.0:8000")?
    .run()
}
