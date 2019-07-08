extern crate oauth2;
extern crate url;

use oauth2::{
    AuthorizationCode,
    AuthUrl,
    ClientId,
    ClientSecret,
    CsrfToken,
    RedirectUrl,
    Scope,
    TokenResponse,
    TokenUrl
};
use oauth2::basic::BasicClient;

use url::Url;

use actix_web::{Responder};
use actix_web::{web, http};
use actix_identity::Identity;
use actix_session::Session;

use askama::Template;

use super::{KachiClashError, BaseTemplate, Result};
use crate::{AppState, Config};
use crate::data::player;
use crate::external::discord;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    base: BaseTemplate
}

pub fn index(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let s = LoginTemplate {
        base: BaseTemplate::new(&state, &identity)?
    }.render().unwrap();
    Ok(web::HttpResponse::Ok().content_type("text/html").body(s))
}

pub fn discord(state: web::Data<AppState>, session: Session) -> impl Responder {
    let config = &state.config;
    let client = make_discord_oauth_client(&config);

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

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

pub fn discord_redirect(query: web::Query<OAuthRedirectQuery>, state: web::Data<AppState>, session: Session, id: Identity) -> std::result::Result<impl Responder, actix_web::Error> {

    match session.get::<String>("discord_csrf")? {
        Some(ref session_csrf) if *session_csrf == query.state => {
            let oauth_client = make_discord_oauth_client(&state.config);
            let auth_code = AuthorizationCode::new(query.code.to_owned());
            let token_res = oauth_client.exchange_code(auth_code)
                .request(oauth2::reqwest::http_client)
                .map_err(|e| {
                    warn!("error exchanging auth code: {:?}", e);
                    KachiClashError::ExternalServiceError
                })?;
            let user_info = discord::get_logged_in_user_info(token_res.access_token())
                .map_err(|e| {
                    warn!("error getting logged in user info from discord: {:?}", e);
                    KachiClashError::ExternalServiceError
                })?;
            let player_id = player::player_for_discord_user(&state.db, user_info)
                .map_err(|err| {
                    warn!("error creating player for discord login: {:?}", err);
                    KachiClashError::DatabaseError
                })?;

            id.remember(player_id.to_string());

            Ok(web::HttpResponse::SeeOther()
                .set_header(http::header::LOCATION, "/")
                .finish())
        },
        Some(_) | None => Err(KachiClashError::CSRFError.into())
    }
}

pub fn logout(id: Identity) -> impl Responder {
    id.forget();
    web::HttpResponse::Ok().json(())
}


fn make_discord_oauth_client(config: &Config) -> BasicClient {
    BasicClient::new(
        ClientId::new(config.discord_client_id.to_owned()),
        Some(ClientSecret::new(config.discord_client_secret.to_owned())),
        AuthUrl::new(Url::parse("https://discordapp.com/api/oauth2/authorize").unwrap()),
        Some(TokenUrl::new(Url::parse("https://discordapp.com/api/oauth2/token").unwrap()))
    )
    .set_redirect_url(RedirectUrl::new(Url::parse("http://localhost:8000/login/discord_redirect").unwrap()))
}
