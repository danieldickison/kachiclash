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


use crate::{AppState, Config};
use actix_web::Responder;
use actix_web::{web, http};
use actix_identity::Identity;

use askama::Template;

#[derive(Template)]
#[template(path = "login.html")]
struct LoginTemplate {
    discord_authorize_url: String
}

pub fn login() -> impl Responder {
    let s = LoginTemplate {
        discord_authorize_url: "".to_string()
    }.render().unwrap();
    web::HttpResponse::Ok().content_type("text/html").body(s)

}

pub fn login_with_discord(state: web::Data<AppState>) -> impl Responder {
    let config = &state.config;
    let client = make_discord_oauth_client(&config);

    let (auth_url, csrf_token) = client
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url();

    web::HttpResponse::SeeOther()
        .set_header(http::header::LOCATION, auth_url.to_string())
        .finish()
}

pub fn logged_in_with_discord(state: web::Data<AppState>) -> impl Responder {
    web::HttpResponse::SeeOther()
        .set_header(http::header::LOCATION, "/")
        .finish()
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
