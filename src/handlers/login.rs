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
use crate::{AppState};
use crate::data::player;
use crate::external::discord;

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
    let (auth_url, csrf_token) = discord::authorize_url(&state.config);

    session.set("discord_csrf", csrf_token)
        .expect("could not set discord_csrf session value");

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

    let mut db = state.db.lock().unwrap();

    match session.get::<String>("discord_csrf").unwrap_or(None) {
        Some(ref session_csrf) if *session_csrf == query.state => {
            let auth_code = AuthorizationCode::new(query.code.to_owned());
            let token_res = discord::exchange_code(&state.config, auth_code)
                .map_err(|e| {
                    warn!("error exchanging auth code for access token from discord: {:?}", e);
                    HandlerError::ExternalServiceError
                })?;
            let user_info = discord::get_logged_in_user_info(token_res.access_token())
                .map_err(|e| {
                    warn!("error getting logged in user info from discord: {:?}", e);
                    HandlerError::ExternalServiceError
                })?;
            let player_id = player::player_for_discord_user(&mut db, user_info)
                .map_err(|err| {
                    warn!("error creating player for discord login: {:?}", err);
                    HandlerError::DatabaseError
                })?;

            id.remember(player_id.to_string());

            Ok(web::HttpResponse::SeeOther()
                .set_header(http::header::LOCATION, "/")
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

