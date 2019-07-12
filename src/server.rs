use super::{AppState, Config};
use super::{data, handlers};

use std::convert::TryInto;
use std::process::Command;

use actix_web::{web, middleware, HttpServer, App, HttpResponse};
use actix_web::cookie::SameSite;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_session::{CookieSession};
use actix_files::Files;
use chrono::Duration;


pub fn run(config: Config) -> std::io::Result<()> {

    let config2 = config.clone();
    let session_secret: [u8; 32] = config.session_secret.as_bytes().try_into().expect("session key should be 32 utf8 bytes");

    if config.is_dev() {
        info!("starting sass --watch scss/:public/css/");
        // Not sure if we need to .wait on the child process or kill it manually. On my mac it seems to be unnecessary.
        let _sass = Command::new("sass")
            .arg("--watch")
            .arg("scss/:public/css/")
            .spawn()
            .expect("run sass");
    }

    info!("starting server at {}:{}", config2.host, config2.port);
    HttpServer::new(move || App::new()
        .data(AppState {
            config: config.clone(),
            db: data::make_conn(&config.db_path),
        })

        .wrap(middleware::Logger::default())
        .wrap(IdentityService::new(
            CookieIdentityPolicy::new(&session_secret)
              .secure(!config.is_dev())
              .same_site(SameSite::Lax)
              .max_age_time(Duration::days(3650)))
        )
        .wrap(CookieSession::signed(&session_secret).secure(config.env != "dev"))
        .wrap(middleware::DefaultHeaders::new().header("Content-Type", "text/html; charset=utf-8"))

        .service(Files::new("/static", "public"))
        .service(web::resource("/").to(handlers::index))
        .service(web::resource("/login").to(handlers::login::index))
        .service(web::resource("/logout").to(handlers::login::logout))
        .service(web::resource("/login/discord").to(handlers::login::discord))
        .service(web::resource("/login/discord_redirect").to(handlers::login::discord_redirect))
        .service(web::resource("/basho").to(handlers::basho::basho_list))
        .service(web::resource("/basho/{basho_id}").to(handlers::basho::basho))
        .service(web::resource("/basho/{basho_id}/picks")
                    .route(web::post().to(handlers::basho::save_picks)))
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
