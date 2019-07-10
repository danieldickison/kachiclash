use super::{AppState, Config};
use super::{data, handlers};

use std::convert::TryInto;

use actix_web::{web, HttpServer, App, HttpResponse};
use actix_web::middleware::Logger;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_session::{CookieSession};
use actix_files::Files;


pub fn run(config: Config) -> std::io::Result<()> {

    let config2 = config.clone();
    let session_secret: [u8; 32] = config.session_secret.as_bytes().try_into().expect("session key should be 32 utf8 bytes");

    info!("starting server at {}:{}", config2.host, config2.port);
    HttpServer::new(move || App::new()
        .data(AppState {
            config: config.clone(),
            db: data::make_conn(&config.db_path),
        })
        .wrap(Logger::default())
        .wrap(IdentityService::new(
            CookieIdentityPolicy::new(&session_secret)
              .secure(config.env != "dev")))
        .wrap(CookieSession::signed(&session_secret).secure(config.env != "dev"))
        .service(Files::new("/static", "public"))
        .service(web::resource("/").to(handlers::index))
        .service(web::resource("/login").to(handlers::login::index))
        .service(web::resource("/logout").to(handlers::login::logout))
        .service(web::resource("/login/discord").to(handlers::login::discord))
        .service(web::resource("/login/discord_redirect").to(handlers::login::discord_redirect))
        .service(web::resource("/basho").to(handlers::basho::basho_list))
        .service(web::resource("/basho/{basho_id}").to(handlers::basho::basho))
        .service(
            web::scope("/db")
                .service(web::resource("/player").to(handlers::list_players))
        )
        .default_service(
            web::route().to(|| HttpResponse::NotFound())
        )
    )
    .bind(("0.0.0.0", config2.port))?
    .run()
}
