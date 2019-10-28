
use crate::Config;

use std::fmt;
use failure::Error;
use reqwest;
use url::Url;

use oauth2::{
    AccessToken,
    AuthorizationCode,
    AuthUrl,
    ClientId,
    ClientSecret,
    CsrfToken,
    RedirectUrl,
    Scope,
    TokenUrl
};
use oauth2::basic::{BasicClient, BasicTokenResponse};


const URL_BASE: &str = "https://discordapp.com/api/v6/";
const IMG_BASE: &str = "https://cdn.discordapp.com/";


fn make_oauth_client(config: &Config) -> BasicClient {
    let mut redirect_url = config.url();
    redirect_url.set_path("login/discord_redirect");
    
    BasicClient::new(
        ClientId::new(config.discord_client_id.to_owned()),
        Some(ClientSecret::new(config.discord_client_secret.to_owned())),
        AuthUrl::new(Url::parse("https://discordapp.com/api/oauth2/authorize").unwrap()),
        Some(TokenUrl::new(Url::parse("https://discordapp.com/api/oauth2/token").unwrap()))
    )
    .set_redirect_url(RedirectUrl::new(redirect_url))
}

pub fn authorize_url(config: &Config) -> (Url, CsrfToken) {
    make_oauth_client(&config)
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("identify".to_string()))
        .url()
}

pub fn exchange_code(config: &Config, auth_code: AuthorizationCode) -> Result<BasicTokenResponse, Error> {
    make_oauth_client(&config)
        .exchange_code(auth_code)
        .request(oauth2::reqwest::http_client)
        .map_err(|e| e.into())
}

#[derive(Debug, Deserialize, Clone)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub discriminator: String, // 4-digits
    pub avatar: Option<String>,
}

pub fn get_logged_in_user_info(access_token: &AccessToken) -> Result<UserInfo, Error> {
    let req = reqwest::Client::new()
        .get(url("users/@me"))
        .bearer_auth(access_token.secret())
        .header("User-Agent", "KachiClash (http://kachiclash.com, 1)");
    debug!("sending request: {:?}", req);
    let mut res = req.send()?;
    debug!("response: {:?}", res);
    if res.status().is_success() {
        res.json().map_err(|e| e.into())
    } else {
        debug!("body: {}", res.text()?);
        Err(format_err!("getting logged in user info failed with http status: {}", res.status()))
    }
}

fn url(path: &str) -> Url {
    Url::parse(URL_BASE).unwrap().join(path).expect("bad discord api path")
}

pub enum ImageExt {
    PNG,
    // JPEG,
}

impl fmt::Display for ImageExt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", match self {
            ImageExt::PNG => "png",
            // ImageExt::JPEG => "jpg",
        })
    }
}

pub enum ImageSize {
    TINY    = 64,
    // SMALL   = 128,
    // MEDIUM  = 512,
    // LARGE   = 1024,
}

pub fn avatar_url(user_info: &UserInfo, ext: ImageExt, size: ImageSize) -> Url {
    let base = Url::parse(IMG_BASE).unwrap();
    if let Some(hash) = &user_info.avatar {
        base.join(&format!("avatars/{}/{}.{}?size={}", user_info.id, hash, ext, size as i32)[..]).unwrap()
    } else {
        let discrim = u16::from_str_radix(&user_info.discriminator[..], 10).unwrap_or(0) % 5;
        base.join(&format!("embed/avatars/{}.png?size={}", discrim, size as i32)[..]).unwrap()
    }
}
