use failure::Error;
use reqwest;
use url::Url;

use oauth2::{AccessToken};

const URL_BASE: &str = "https://discordapp.com/api/v6/";

#[derive(Debug, Deserialize)]
pub struct UserInfo {
    pub id: String,
    pub username: String,
    pub discriminator: String,
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
