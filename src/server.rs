use crate::data::DbConn;

use super::{data, handlers};
use super::{AppState, Config};

use std::convert::TryInto;
use std::process::Command;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_rt::Arbiter;
use actix_rt::time::interval;
use actix_session::CookieSession;
use actix_web::cookie::SameSite;
use actix_web::dev::Server;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use std::time::Duration;
use std::cmp::max;

pub async fn run(config: Config) -> std::io::Result<()> {
    let is_dev = config.is_dev();
    let port = config.port;
    let session_secret: [u8; 32] = config
        .session_secret
        .as_bytes()
        .try_into()
        .expect("session key should be 32 utf8 bytes");
    let db_mutex = data::make_conn(&config.db_path);
    let db_mutex2 = db_mutex.clone();
    let workers;
    if is_dev {
        workers = 2
    } else {
        workers = max(num_cpus::get(), 4)
    }

    info!("starting server at {}:{}", config.host, config.port);
    let server = HttpServer::new(move || {
        let mut app = App::new()
            .data(AppState {
                config: config.clone(),
                db: db_mutex.clone(),
            })
            .wrap(middleware::Logger::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&session_secret)
                    .secure(!config.is_dev())
                    .same_site(SameSite::Lax)
                    .max_age(10 * 365 * 24 * 60 * 60),
            ))
            .wrap(CookieSession::signed(&session_secret).secure(config.env != "dev"))
            .wrap(
                middleware::DefaultHeaders::new()
                    .header("Content-Type", "text/html; charset=utf-8"),
            )
            .service(Files::new("/static", &config.static_path).prefer_utf8(true))
            .service(web::resource("/").to(handlers::index::index))
            .service(web::resource("/logout").to(handlers::login::logout))
            .service(
                web::scope("/login")
                    .service(web::resource("").to(handlers::login::index))
                    .service(web::resource("/discord").to(handlers::login::discord))
                    .service(
                        web::resource("/discord_redirect").to(handlers::login::discord_redirect),
                    )
                    .service(web::resource("/google").to(handlers::login::google))
                    .service(web::resource("/google_redirect").to(handlers::login::google_redirect))
                    .service(web::resource("/reddit").to(handlers::login::reddit))
                    .service(
                        web::resource("/reddit_redirect").to(handlers::login::reddit_redirect),
                    ),
            )
            .service(
                web::resource("/settings")
                    .route(web::get().to(handlers::settings::settings_page))
                    .route(web::post().to(handlers::settings::settings_post)),
            )
            .service(
                web::resource("/stats")
                    .route(web::get().to(handlers::stats::stats_page))
            )
            .service(
                web::scope("/basho/{basho_id}")
                    .service(web::resource("").to(handlers::basho::basho))
                    .service(
                        web::resource("/edit")
                            .route(web::get().to(handlers::admin::edit_basho_page))
                            .route(web::post().to(handlers::admin::edit_basho_post)),
                    )
                    .service(
                        web::resource("/picks").route(web::post().to(handlers::basho::save_picks)),
                    )
                    .service(
                        web::resource("/day/{day}")
                            .route(web::get().to(handlers::admin::torikumi_page))
                            .route(web::post().to(handlers::admin::torikumi_post)),
                    )
                    .service(
                        web::resource("/bestow_emperors_cup")
                            .route(web::post().to(handlers::admin::bestow_emperors_cup)),
                    )
                    .service(
                        web::resource("/revoke_emperors_cup")
                            .route(web::post().to(handlers::admin::revoke_emperors_cup)),
                    )
                    .service(
                        web::resource("/finalize")
                            .route(web::post().to(handlers::admin::finalize_basho)),
                    ),
            )
            .service(
                web::scope("/player")
                    .service(web::resource("").to(handlers::admin::list_players))
                    .service(web::resource("/{player_id}").to(handlers::player::player))
                    .service(
                        web::resource("/update_images")
                            .route(web::post().to(handlers::admin::update_user_images)),
                    ),
            )
            .default_service(web::route().to(default_not_found));

        if config.is_dev() {
            app = app.service(Files::new("/scss", "scss"));
        }
        app
    })
    .workers(workers)
    .bind(("0.0.0.0", port))?
    .run();

    Arbiter::spawn(DbWatchdog::new(&db_mutex2, &server).run());

    if is_dev {
        info!("starting sass --watch scss/:public/css/");
        // Not sure if we need to .wait on the child process or kill it manually. On my mac it seems to be unnecessary.
        let _sass = Command::new("sass")
            .arg("--watch")
            .arg("scss/:public/css/")
            .spawn()
            .expect("run sass");
    }

    server.await
}

async fn default_not_found() -> Result<HttpResponse, handlers::HandlerError> {
    Err(handlers::HandlerError::NotFound("Page".to_string()))
}


struct DbWatchdog {
    db: DbConn,
    server: Server,
}

impl DbWatchdog {
    fn new(db: &DbConn, server: &Server) -> Self {
        Self {
            db: db.clone(),
            server: server.clone(),
        }
    }

    async fn run(self) -> () {
        let mut interval = interval(Duration::from_secs(10));
        while !self.db.is_poisoned() {
            // debug!("db watchdog: ok");
            interval.tick().await;
        }
        error!("watchdog: db mutex is poisoned; graceful shutdown");
        self.server.stop(true).await;
    }
}
