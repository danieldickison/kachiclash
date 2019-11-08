use rusqlite::Transaction;
use oauth2::{CsrfToken, AuthorizationCode, AccessToken, Scope};
use oauth2::basic::{BasicTokenResponse, BasicClient};
use url::Url;
use failure::Error;
use serde::de::DeserializeOwned;
use chrono::{DateTime, Utc};
use rand;

use crate::Config;
use crate::data::PlayerId;
use std::fmt::Debug;

pub mod discord;
pub mod google;
pub mod reddit;

pub trait UserInfo {
    fn update_existing_player(&self, txn: &Transaction, mod_date: DateTime<Utc>)
        -> Result<Option<PlayerId>, rusqlite::Error>;

    fn insert_into_db(&self, txn: &Transaction, mod_date: DateTime<Utc>, player_id: PlayerId)
        -> Result<usize, rusqlite::Error>;

    fn name_suggestion(&self) -> Option<String>;

    fn anon_name_suggestion(&self) -> String {
        format!("anon{:05}", rand::random::<u16>())
    }
}

pub trait AuthProvider: Debug {
    type UserInfo: UserInfo + DeserializeOwned;
    const SCOPES: &'static [&'static str];
    const USER_INFO_URL: &'static str;

    fn make_oauth_client(&self, config: &Config) -> BasicClient;

    fn authorize_url(&self, config: &Config) -> (Url, CsrfToken) {
        let client = self.make_oauth_client(&config);
        let mut req = client.authorize_url(CsrfToken::new_random);
        for scope in Self::SCOPES {
            req = req.add_scope(Scope::new(scope.to_string()));
        }
        req.url()
    }

    fn exchange_code(&self, config: &Config, auth_code: AuthorizationCode) -> Result<BasicTokenResponse, Error> {
        self.make_oauth_client(&config)
            .exchange_code(auth_code)
            .request(oauth2::reqwest::http_client)
            .map_err(|e| e.into())
    }

    fn get_logged_in_user_info(&self, access_token: &AccessToken) -> Result<Self::UserInfo, Error> {
        let req = reqwest::Client::new()
            .get(Self::USER_INFO_URL)
            .bearer_auth(access_token.secret())
            .header("User-Agent", "KachiClash (http://kachiclash.com, 1)");
        //debug!("sending request: {:?}", req); // Note: this logs sensitive data
        let mut res = req.send()?;
        //debug!("response: {:?}", res); // Note: this logs sensitive data
        if res.status().is_success() {
            res.json().map_err(|e| e.into())
        } else {
            debug!("body: {}", res.text()?);
            Err(format_err!("getting logged in user info failed with http status: {}", res.status()))
        }
    }
}
