extern crate oauth2;
extern crate url;

use oauth2::{
    AuthorizationCode,
    TokenResponse
};

use actix_web::{Responder};
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

pub fn index(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    let s = LoginTemplate {
        base: BaseTemplate::new(&db, &identity)?
    }.render().unwrap();
    Ok(web::HttpResponse::Ok().body(s))
}

pub fn discord(state: web::Data<AppState>, session: Session) -> impl Responder {
    oauth_login(&state.config, session, DiscordAuthProvider)
}

pub fn google(state: web::Data<AppState>, session: Session) -> impl Responder {
    oauth_login(&state.config, session, GoogleAuthProvider)
}

pub fn reddit(state: web::Data<AppState>, session: Session) -> impl Responder {
    oauth_login(&state.config, session, RedditAuthProvider)
}

fn oauth_login(config: &Config, session: Session, provider: impl AuthProvider) -> impl Responder {
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

pub fn discord_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> Result<impl Responder> {
    oauth_redirect(&query, state, session, id, DiscordAuthProvider)
}

pub fn google_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> Result<impl Responder> {
    oauth_redirect(&query, state, session, id, GoogleAuthProvider)
}

pub fn reddit_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> Result<impl Responder> {
    oauth_redirect(&query, state, session, id, RedditAuthProvider)
}

fn oauth_redirect(query: &OAuthRedirectQuery, state: web::Data<AppState>, session: Session, id: Identity, provider: impl AuthProvider)
    -> Result<impl Responder> {

    let mut db = state.db.lock().unwrap();

    match session.get::<String>("oauth_csrf").unwrap_or(None) {
        Some(ref session_csrf) if *session_csrf == query.state => {
            let auth_code = AuthorizationCode::new(query.code.to_owned());
            let token_res = provider.exchange_code(&state.config, auth_code)
                .map_err(|e| {
                    warn!("error exchanging auth code for access token from discord: {:?}", e);
                    HandlerError::ExternalServiceError
                })?;
            let user_info = provider.get_logged_in_user_info(token_res.access_token())
                .map_err(|e| {
                    warn!("error getting logged in user info from discord: {:?}", e);
                    HandlerError::ExternalServiceError
                })?;
            let (player_id, is_new) = player::player_id_with_external_user(&mut db, user_info)
                .map_err(|err| {
                    warn!("error creating player for discord login: {:?}", err);
                    HandlerError::DatabaseError(err.into())
                })?;

            id.remember(player_id.to_string());
            session.remove("oauth_csrf");

            Ok(web::HttpResponse::SeeOther()
                .set_header(http::header::LOCATION, if is_new {"/settings"} else {"/"})
                .finish())
        },
        Some(_) | None => {
            warn!("bad CSRF token received in discord oauth redirect endpoint");
            Err(HandlerError::CSRFError.into())
        }
    }
}

pub fn logout(id: Identity) -> impl Responder {
    id.forget();
    web::HttpResponse::SeeOther()
        .set_header(http::header::LOCATION, "/")
        .finish()
}

