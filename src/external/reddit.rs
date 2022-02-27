use oauth2::{RedirectUrl, TokenUrl, ClientId, ClientSecret, AuthUrl};
use oauth2::basic::BasicClient;
use rusqlite::{Transaction, Error};
use chrono::{Utc, DateTime};
use async_trait::async_trait;

use crate::Config;
use crate::data::PlayerId;
use super::AuthProvider;
use crate::external::UserInfo;

#[derive(Debug)]
pub struct RedditAuthProvider;

#[async_trait]
impl AuthProvider for RedditAuthProvider {
    fn service_name(&self) -> &'static str {
        "Reddit"
    }

    fn logged_in_user_info_url(&self) -> &'static str {
        "https://oauth.reddit.com/api/v1/me"
    }

    fn oauth_scopes(&self) -> &'static [&'static str] {
        &["identity"]
    }

    fn make_oauth_client(&self, config: &Config) -> BasicClient {
        let mut redirect_url = config.url();
        redirect_url.set_path("login/reddit_redirect");

        BasicClient::new(
            ClientId::new(config.reddit_client_id.to_owned()),
            Some(ClientSecret::new(config.reddit_client_secret.to_owned())),
            AuthUrl::new("https://www.reddit.com/api/v1/authorize?duration=temporary".to_string()).unwrap(),
            Some(TokenUrl::new("https://www.reddit.com/api/v1/access_token".to_string()).unwrap())
        )
        .set_redirect_uri(RedirectUrl::from_url(redirect_url))
    }

    fn make_user_info_url(&self, user_id: &str) -> String {
        format!("https://oauth.reddit.com/api/v1/user/{}/about", user_id)
    }

    async fn parse_user_info_response(&self, res: reqwest::Response) -> anyhow::Result<Box<dyn UserInfo>> {
        Ok(Box::new(res.json::<RedditUserInfo>().await?))
    }

    fn player_id_to_user_id_mapping_sql(&self) -> &'static str {
        "
            SELECT player_id, id AS user_id
            FROM player_reddit WHERE player_id = ?
        "
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedditUserInfo {
    pub id: String,
    pub name: String,
    pub icon_img: Option<String>, // url
}

impl UserInfo for RedditUserInfo {
    fn update_existing_player(&self, txn: &Transaction, mod_date: DateTime<Utc>)
        -> Result<Option<PlayerId>, Error> {

        //debug!("reddit user info: {:?}", self);

        match txn
            .prepare("SELECT player_id, name, icon_img FROM player_reddit WHERE id = ?")?
            .query_map(
                params![self.id],
                |row| -> Result<(PlayerId, String, Option<String>), _> {
                    Ok((row.get("player_id")?,
                        row.get("name")?,
                        row.get("icon_img")?,
                    ))
                }
            )?
            .next() {

            None => Ok(None),
            Some(Ok((player_id, name, icon_img))) => {
                if name != self.name || icon_img != self.icon_img {
                    txn.execute("
                            UPDATE player_reddit
                            SET name = ?, icon_img = ?, mod_date = ?
                            WHERE id = ?
                        ",
                                params![self.name, self.icon_img, mod_date, self.id])?;
                }
                Ok(Some(player_id))
            },
            Some(Err(e)) => Err(e),
        }

    }

    fn insert_into_db(&self, txn: &Transaction, mod_date: DateTime<Utc>, player_id: PlayerId)
        -> Result<usize, rusqlite::Error> {
        txn.execute("
            INSERT INTO player_reddit (player_id, id, name, icon_img, mod_date)
            VALUES (?, ?, ?, ?, ?)",
        params![player_id, self.id, self.name, self.icon_img, mod_date])
    }

    fn name_suggestion(&self) -> Option<String> {
        Some(self.name.to_owned())
    }
}
