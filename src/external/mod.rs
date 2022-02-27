use rusqlite::Transaction;
use oauth2::{CsrfToken, AuthorizationCode, AccessToken, Scope};
use oauth2::basic::{BasicTokenResponse, BasicClient};
use url::Url;
use async_trait::async_trait;
use anyhow::{Result, anyhow};
use chrono::{DateTime, Utc};

use crate::Config;
use crate::data::PlayerId;
use std::fmt::Debug;

pub mod discord;
pub mod google;
pub mod reddit;

pub enum ImageSize {
    Tiny    = 64,
    // SMALL   = 128,
    Medium  = 512,
    // LARGE   = 1024,
}

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

#[async_trait]
pub trait AuthProvider: Send + Sync + Debug {
    fn service_name(&self) -> &'static str;
    fn logged_in_user_info_url(&self) -> &'static str;
    fn oauth_scopes(&self) -> &'static [&'static str];
    fn make_oauth_client(&self, config: &Config) -> BasicClient;
    fn make_user_info_url(&self, user_id: &str) -> String;
    async fn parse_user_info_response(&self, res: reqwest::Response) -> anyhow::Result<Box<dyn UserInfo>>;
    fn player_id_to_user_id_mapping_sql(&self) -> &'static str;

    fn authorize_url(&self, config: &Config) -> (Url, CsrfToken) {
        let client = self.make_oauth_client(config);
        let mut req = client.authorize_url(CsrfToken::new_random);
        for &scope in self.oauth_scopes() {
            req = req.add_scope(Scope::new(scope.to_string()));
        }
        req.url()
    }

    async fn exchange_code(&self, config: &Config, auth_code: AuthorizationCode)
        -> anyhow::Result<BasicTokenResponse> {

        self.make_oauth_client(config)
            .exchange_code(auth_code)
            .request_async(oauth2::reqwest::async_http_client)
            .await
            .map_err(|e| anyhow!("oauth code exchange error: {}", e))
    }

    async fn get_logged_in_user_info(&self, access_token: &AccessToken)
        -> anyhow::Result<Box<dyn UserInfo>> {

        let req = reqwest::Client::new()
            .get(self.logged_in_user_info_url())
            .bearer_auth(access_token.secret())
            .header("User-Agent", "KachiClash (http://kachiclash.com, 1)");
        //debug!("sending request: {:?}", req); // Note: this logs sensitive data
        let res = req.send().await?;
        let status = res.status();
        //debug!("response: {:?}", res); // Note: this logs sensitive data
        if status.is_success() {
            self.parse_user_info_response(res).await
        } else {
            debug!("body: {}", res.text().await?);
            Err(anyhow!("getting logged in user info failed with http status: {}", status))
        }
    }

    async fn get_user_info(&self, access_token: &AccessToken, user_id: &str)
        -> anyhow::Result<Box<dyn UserInfo>> {

        let req = reqwest::Client::new()
            .get(self.make_user_info_url(user_id).as_str())
            .bearer_auth(access_token.secret())
            .header("User-Agent", "KachiClash (http://kachiclash.com, 1)");
        //debug!("sending request: {:?}", req); // Note: this logs sensitive data
        let res = req.send().await?;
        let status = res.status();
        //debug!("response: {:?}", res); // Note: this logs sensitive data
        if status.is_success() {
            self.parse_user_info_response(res).await
        } else {
            debug!("body: {}", res.text().await?);
            Err(anyhow!("getting user info for {} {} failed with http status: {}", self.service_name(), user_id, status))
        }
    }
}
