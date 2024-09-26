use super::handlers;
use super::AppState;
use crate::data::DbConn;

use std::convert::TryInto;
use std::process::Command;

use actix_files::Files;
use actix_identity::IdentityMiddleware;
use actix_session::config::PersistentSession;
use actix_session::storage::CookieSessionStore;
use actix_session::SessionMiddleware;
use actix_web::cookie::Key;
use actix_web::dev::ServerHandle;
use actix_web::rt::time::interval;
use actix_web::{middleware, web, App, HttpResponse, HttpServer};
use anyhow::anyhow;
use std::cmp::max;
use std::time::Duration;
use tokio::task::spawn;

pub async fn run(app_state: &AppState) -> anyhow::Result<()> {
    let config = app_state.config.clone();
    let is_dev = config.is_dev();
    let port = config.port;
    let session_secret: [u8; 64] = config
        .session_secret
        .as_bytes()
        .try_into()
        .expect("session key should be 64 utf8 bytes");
    let db_mutex = app_state.db.clone();
    let workers;
    let static_ttl;
    if is_dev {
        workers = 2;
        static_ttl = 60;
    } else {
        workers = max(num_cpus::get(), 4);
        static_ttl = 3600;
    }
    let app_data = web::Data::new(app_state.clone());
    let year = actix_web::cookie::time::Duration::days(365);

    info!("starting server at {}:{}", config.host, config.port);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::clone(&app_data))
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            .wrap(IdentityMiddleware::builder().build())
            .wrap(
                SessionMiddleware::builder(
                    CookieSessionStore::default(),
                    Key::from(&session_secret),
                )
                .session_lifecycle(PersistentSession::default().session_ttl(10 * year))
                .cookie_content_security(if is_dev {
                    actix_session::config::CookieContentSecurity::Signed
                } else {
                    actix_session::config::CookieContentSecurity::Private
                })
                .cookie_secure(!is_dev)
                .build(),
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
            .service(handlers::index::index)
            .service(handlers::index::pwa)
            .service(handlers::login::logout)
            .service(
                web::scope("/login")
                    .service(handlers::login::index)
                    .service(handlers::login::discord)
                    .service(handlers::login::discord_redirect)
                    .service(handlers::login::google)
                    .service(handlers::login::google_redirect)
                    .service(handlers::login::reddit)
                    .service(handlers::login::reddit_redirect),
            )
            .service(handlers::settings::settings_page)
            .service(handlers::settings::settings_post)
            .service(
                web::scope("/push")
                    .service(handlers::push::check)
                    .service(handlers::push::test)
                    .service(handlers::push::trigger),
            )
            .service(handlers::stats::stats_page)
            .service(
                web::scope("/basho/{basho_id}")
                    .service(handlers::basho::basho)
                    .service(handlers::basho::save_picks)
                    .service(handlers::admin::edit_basho_page)
                    .service(handlers::admin::edit_basho_post)
                    .service(handlers::admin::torikumi_page)
                    .service(handlers::admin::torikumi_post)
                    .service(handlers::admin::finalize_basho)
                    .service(handlers::admin::backfill_player_ranks),
            )
            .service(
                web::scope("/heya/{heya_id}")
                    .service(handlers::heya::page)
                    .service(handlers::heya::edit),
            )
            .service(handlers::heya::create)
            .service(handlers::heya::list)
            .service(handlers::admin::list_players)
            .service(handlers::player::player_page)
            .service(handlers::admin::update_user_images)
            .default_service(web::route().to(default_not_found))
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
            .arg("public/scss/:public/css/")
            .spawn()
            .expect("run sass");

        info!("starting npx tsc --watch");
        // Not sure if we need to .wait on the child process or kill it manually. On my mac it seems to be unnecessary.
        let _sass = Command::new("npx")
            .arg("tsc")
            .arg("--watch")
            .arg("--preserveWatchOutput")
            .spawn()
            .expect("run tsc");
    }

    server.await?;
    Err(anyhow!("Server exited"))
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
