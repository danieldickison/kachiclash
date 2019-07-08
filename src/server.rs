use super::{AppState, Config};
use super::{data, handlers};

use std::convert::TryInto;

use actix_web::{web, HttpServer, App, HttpResponse};
use actix_web::middleware::Logger;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_session::{CookieSession};


pub fn run(config: Config) -> std::io::Result<()> {

    let config2 = config.clone();
    let session_secret: [u8; 32] = config.session_secret.as_bytes().try_into().expect("session key should be 32 utf8 bytes");

    info!("starting server on localhost:8000");
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
        .service(web::resource("/").to(handlers::index))
        .service(web::resource("/login").to(handlers::login::index))
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
    .bind(("0.0.0.0", config2.port))?
    .run()
}
