use crate::data::push::PushBuilder;
use crate::data::DbConn;

use super::{data, handlers};
use super::{AppState, Config};

use std::convert::TryInto;
use std::process::Command;

use actix_files::Files;
use actix_identity::{CookieIdentityPolicy, IdentityService};
use actix_session::CookieSession;
use actix_web::cookie::SameSite;
use actix_web::dev::ServerHandle;
use actix_web::rt::time::interval;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use std::cmp::max;
use std::time::Duration;
use tokio::task::spawn;

pub async fn run(config: Config) -> anyhow::Result<()> {
    let is_dev = config.is_dev();
    let port = config.port;
    let session_secret: [u8; 32] = config
        .session_secret
        .as_bytes()
        .try_into()
        .expect("session key should be 32 utf8 bytes");
    let db_mutex = data::make_conn(&config.db_path);
    let workers;
    let static_ttl;
    if is_dev {
        workers = 2;
        static_ttl = 60;
    } else {
        workers = max(num_cpus::get(), 4);
        static_ttl = 3600;
    }
    let app_data = web::Data::new(AppState {
        config: config.clone(),
        db: db_mutex.clone(),
        push: PushBuilder::with_base64_private_key(&config.vapid_private_key)?,
    });

    info!("starting server at {}:{}", config.host, config.port);
    let server = HttpServer::new(move || {
        let mut app = App::new()
            .app_data(web::Data::clone(&app_data))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(IdentityService::new(
                CookieIdentityPolicy::new(&session_secret)
                    .domain(&config.host)
                    .secure(!config.is_dev())
                    .same_site(SameSite::Lax)
                    .max_age_secs(10 * 365 * 24 * 60 * 60),
            ))
            .wrap(
                CookieSession::signed(&session_secret)
                    .domain(&config.host)
                    .secure(config.env != "dev")
                    .same_site(SameSite::Lax),
            )
            .wrap(
                middleware::DefaultHeaders::new().add(("Content-Type", "text/html; charset=utf-8")),
            )
            .service(
                web::scope("/static")
                    .wrap(
                        middleware::DefaultHeaders::new()
                            .add(("Cache-Control", format!("max-age={static_ttl}")))
                            .add(("Service-Worker-Allowed", "/")),
                    )
                    .service(Files::new("/", &config.static_path).prefer_utf8(true)),
            )
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
                web::scope("/push")
                    .service(handlers::push::register)
                    .service(handlers::push::check)
                    .service(handlers::push::test),
            )
            .service(web::resource("/stats").route(web::get().to(handlers::stats::stats_page)))
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

    spawn(DbWatchdog::new(&db_mutex, &server.handle()).run());

    if is_dev {
        info!("starting sass --watch scss/:public/css/");
        // Not sure if we need to .wait on the child process or kill it manually. On my mac it seems to be unnecessary.
        let _sass = Command::new("sass")
            .arg("--watch")
            .arg("scss/:public/css/")
            .spawn()
            .expect("run sass");
    }

    server.await.map_err(|e| e.into())
}

async fn default_not_found() -> Result<HttpResponse, handlers::HandlerError> {
    Err(handlers::HandlerError::NotFound("Page".to_string()))
}

struct DbWatchdog {
    db: DbConn,
    handle: ServerHandle,
}

impl DbWatchdog {
    fn new(db: &DbConn, handle: &ServerHandle) -> Self {
        Self {
            db: db.clone(),
            handle: handle.clone(),
        }
    }

    async fn run(self) {
        let mut interval = interval(Duration::from_secs(10));
        while !self.db.is_poisoned() {
            // debug!("db watchdog: ok");
            interval.tick().await;
        }
        error!("watchdog: db mutex is poisoned; graceful shutdown");
        self.handle.stop(true).await;
    }
}
