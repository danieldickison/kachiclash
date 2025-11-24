extern crate oauth2;
extern crate url;

use askama_web::WebTemplate;
use oauth2::{AuthorizationCode, CsrfToken, TokenResponse};

use actix_identity::Identity;
use actix_session::Session;
use actix_web::{get, http, web, HttpMessage, HttpRequest};
use actix_web::{HttpResponse, Responder};

use askama::Template;
use serde::Serialize;

use super::{BaseTemplate, HandlerError, Result};
use crate::data::player;
use crate::external::discord::DiscordAuthProvider;
use crate::external::google::GoogleAuthProvider;
use crate::external::reddit::RedditAuthProvider;
use crate::external::AuthProvider;
use crate::{AppState, Config};

#[derive(Template, WebTemplate)]
#[template(path = "login.html")]
struct LoginTemplate {
    base: BaseTemplate,
}

#[derive(Serialize)]
pub struct AuthProviderInfo {
    pub display_name: String,
    pub login_url: String,
}

#[derive(Serialize)]
pub struct LookupResponse {
    pub providers: Vec<AuthProviderInfo>,
}

#[get("")]
pub async fn index(
    state: web::Data<AppState>,
    identity: Option<Identity>,
) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    let s = LoginTemplate {
        base: BaseTemplate::new(&db, identity.as_ref(), &state)?,
    }
    .render()
    .unwrap();
    Ok(HttpResponse::Ok().body(s))
}

#[get("/discord")]
pub async fn discord(state: web::Data<AppState>, session: Session) -> HttpResponse {
    oauth_login(&state.config, session, DiscordAuthProvider)
}

#[get("/google")]
pub async fn google(state: web::Data<AppState>, session: Session) -> HttpResponse {
    oauth_login(&state.config, session, GoogleAuthProvider)
}

#[get("/reddit")]
pub async fn reddit(state: web::Data<AppState>, session: Session) -> HttpResponse {
    oauth_login(&state.config, session, RedditAuthProvider)
}

fn oauth_login(config: &Config, session: Session, provider: impl AuthProvider) -> HttpResponse {
    let (auth_url, csrf_token) = provider.authorize_url(config);
    session
        .insert("oauth_csrf", csrf_token)
        .expect("could not set oauth_csrf session value");
    HttpResponse::SeeOther()
        .insert_header((http::header::LOCATION, auth_url.to_string()))
        .finish()
}

#[derive(Deserialize)]
pub struct OAuthRedirectQuery {
    code: String,
    state: String,
}

#[get("/discord_redirect")]
pub async fn discord_redirect(
    request: HttpRequest,
    query: web::Query<OAuthRedirectQuery>,
    state: web::Data<AppState>,
    session: Session,
) -> Result<impl Responder> {
    oauth_redirect(request, &query, state, session, DiscordAuthProvider).await
}

#[get("/google_redirect")]
pub async fn google_redirect(
    request: HttpRequest,
    query: web::Query<OAuthRedirectQuery>,
    state: web::Data<AppState>,
    session: Session,
) -> Result<impl Responder> {
    oauth_redirect(request, &query, state, session, GoogleAuthProvider).await
}

#[get("/reddit_redirect")]
pub async fn reddit_redirect(
    request: HttpRequest,
    query: web::Query<OAuthRedirectQuery>,
    state: web::Data<AppState>,
    session: Session,
) -> Result<impl Responder> {
    oauth_redirect(request, &query, state, session, RedditAuthProvider).await
}

async fn oauth_redirect(
    request: HttpRequest,
    query: &OAuthRedirectQuery,
    state: web::Data<AppState>,
    session: Session,
    provider: impl AuthProvider,
) -> Result<impl Responder> {
    match session.get::<CsrfToken>("oauth_csrf").unwrap_or(None) {
        Some(ref session_csrf) if *session_csrf.secret() == query.state => {
            debug!("exchanging oauth code for access token from {:?}", provider);
            let auth_code = AuthorizationCode::new(query.code.to_owned());
            let token_res = provider
                .exchange_code(&state.config, auth_code)
                .await
                .map_err(|e| {
                    warn!(
                        "error exchanging auth code for access token from {:?}: {:?}",
                        provider, e
                    );
                    HandlerError::ExternalServiceError
                })?;

            debug!("getting logged in user info from {:?}", provider);
            let user_info = provider
                .get_logged_in_user_info(token_res.access_token())
                .await
                .map_err(|e| {
                    warn!(
                        "error getting logged in user info from {:?}: {:?}",
                        provider, e
                    );
                    HandlerError::ExternalServiceError
                })?;
            let (player_id, is_new) =
                player::player_id_with_external_user(&mut state.db.lock().unwrap(), user_info)
                    .map_err(|err| {
                        warn!("error creating player for {:?} login: {:?}", provider, err);
                        HandlerError::DatabaseError(err.into())
                    })?;

            debug!("logged in as player {}, is_new: {}", player_id, is_new);
            Identity::login(&request.extensions(), player_id.to_string())?;
            session.remove("oauth_csrf");

            Ok(HttpResponse::SeeOther()
                .insert_header((
                    http::header::LOCATION,
                    if is_new { "/settings" } else { "/" },
                ))
                .finish())
        }
        Some(_) | None => {
            warn!(
                "bad CSRF token received in {:?} oauth redirect endpoint",
                provider
            );
            trace!("session entries: {:?}", session.entries());
            // session.purge();
            Err(HandlerError::CSRFError)
        }
    }
}

#[derive(serde::Deserialize)]
pub struct UsernameQuery {
    username: String,
}

#[get("/lookup")]
pub async fn lookup(
    state: web::Data<AppState>,
    query: web::Query<UsernameQuery>,
) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();

    // Get current or next basho for rank lookup (doesn't matter much for this lookup)
    let current_basho = crate::data::BashoInfo::current_or_next_basho_id(&db)?;

    match player::Player::with_name(&db, query.username.clone(), current_basho) {
        Ok(Some(player)) => {
            let linked_providers = player.get_linked_auth_providers();

            if linked_providers.is_empty() {
                return Err(HandlerError::Failure(anyhow::anyhow!(
                    "Player has no linked auth providers"
                )));
            }

            let provider_urls: std::collections::HashMap<&str, &str> = [
                ("discord", "/login/discord"),
                ("google", "/login/google"),
                ("reddit", "/login/reddit"),
            ]
            .iter()
            .copied()
            .collect();

            let providers = linked_providers
                .into_iter()
                .map(|(name, display_name)| AuthProviderInfo {
                    display_name: display_name.to_string(),
                    login_url: provider_urls
                        .get(name)
                        .map(|url| url.to_string())
                        .unwrap_or_default(),
                })
                .collect();

            Ok(HttpResponse::Ok().json(LookupResponse { providers }))
        }
        Ok(None) => Err(HandlerError::NotFound(format!(
            "Player '{}' not found",
            query.username
        ))),
        Err(e) => Err(HandlerError::DatabaseError(e.into())),
    }
}

#[get("/logout")]
pub async fn logout(id: Identity) -> impl Responder {
    id.logout();
    HttpResponse::SeeOther()
        .insert_header((http::header::LOCATION, "/"))
        .finish()
}
