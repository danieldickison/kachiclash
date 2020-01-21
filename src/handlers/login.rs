extern crate oauth2;
extern crate url;

use oauth2::{
    AuthorizationCode,
    TokenResponse
};

use actix_web::{Responder, HttpResponse};
use actix_web::{web, http};
use actix_identity::Identity;
use actix_session::Session;

use askama::Template;

use super::{HandlerError, BaseTemplate, Result};
use crate::{AppState, Config};
use crate::data::player;
use crate::external::{AuthProvider};
use crate::external::google::GoogleAuthProvider;
use crate::external::discord::DiscordAuthProvider;
use crate::external::reddit::RedditAuthProvider;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    base: BaseTemplate
}

pub async fn index(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    let s = LoginTemplate {
        base: BaseTemplate::new(&db, &identity)?
    }.render().unwrap();
    Ok(web::HttpResponse::Ok().body(s))
}

pub async fn discord(state: web::Data<AppState>, session: Session) -> HttpResponse {
    oauth_login(&state.config, session, DiscordAuthProvider)
}

pub async fn google(state: web::Data<AppState>, session: Session) -> HttpResponse {
    oauth_login(&state.config, session, GoogleAuthProvider)
}

pub async fn reddit(state: web::Data<AppState>, session: Session) -> HttpResponse {
    oauth_login(&state.config, session, RedditAuthProvider)
}

fn oauth_login(config: &Config, session: Session, provider: impl AuthProvider) -> HttpResponse {
    let (auth_url, csrf_token) = provider.authorize_url(&config);
    session.set("oauth_csrf", csrf_token)
        .expect("could not set oauth_csrf session value");
    web::HttpResponse::SeeOther()
        .set_header(http::header::LOCATION, auth_url.to_string())
        .finish()
}


#[derive(Deserialize)]
pub struct OAuthRedirectQuery {
   code: String,
   state: String,
}

pub async fn discord_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> Result<impl Responder> {
    oauth_redirect(&query, state, session, id, DiscordAuthProvider).await
}

pub async fn google_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> Result<impl Responder> {
    oauth_redirect(&query, state, session, id, GoogleAuthProvider).await
}

pub async fn reddit_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> Result<impl Responder> {
    oauth_redirect(&query, state, session, id, RedditAuthProvider).await
}

async fn oauth_redirect(query: &OAuthRedirectQuery, state: web::Data<AppState>, session: Session, id: Identity, provider: impl AuthProvider + Sync)
    -> Result<impl Responder> {

    let mut db = state.db.lock().unwrap();

    match session.get::<String>("oauth_csrf").unwrap_or(None) {
        Some(ref session_csrf) if *session_csrf == query.state => {
            debug!("exchanging oauth code for access token from {:?}", provider);
            let auth_code = AuthorizationCode::new(query.code.to_owned());
            let token_res = provider.exchange_code(&state.config, auth_code)
                .map_err(|e| {
                    warn!("error exchanging auth code for access token from {:?}: {:?}", provider, e);
                    HandlerError::ExternalServiceError
                })?;

            debug!("getting logged in user info from {:?}", provider);
            let user_info = provider.get_logged_in_user_info(token_res.access_token()).await
                .map_err(|e| {
                    warn!("error getting logged in user info from {:?}: {:?}", provider, e);
                    HandlerError::ExternalServiceError
                })?;
            let (player_id, is_new) = player::player_id_with_external_user(&mut db, user_info)
                .map_err(|err| {
                    warn!("error creating player for {:?} login: {:?}", provider, err);
                    HandlerError::DatabaseError(err.into())
                })?;

            debug!("logged in as player {}, is_new: {}", player_id, is_new);
            id.remember(player_id.to_string());
            session.remove("oauth_csrf");

            Ok(web::HttpResponse::SeeOther()
                .set_header(http::header::LOCATION, if is_new {"/settings"} else {"/"})
                .finish())
        },
        Some(_) | None => {
            warn!("bad CSRF token received in {:?} oauth redirect endpoint", provider);
            Err(HandlerError::CSRFError)
        }
    }
}

pub async fn logout(id: Identity) -> impl Responder {
    id.forget();
    web::HttpResponse::SeeOther()
        .set_header(http::header::LOCATION, "/")
        .finish()
}

