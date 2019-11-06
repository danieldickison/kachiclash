use url::Url;
use oauth2::{RedirectUrl, TokenUrl, ClientId, ClientSecret, AuthUrl};
use oauth2::basic::{BasicClient};
use rusqlite::{Transaction, Error};
use chrono::{Utc, DateTime};

use crate::Config;
use crate::data::{PlayerId};
use super::AuthProvider;
use crate::external::UserInfo;


pub struct RedditAuthProvider;

impl AuthProvider for RedditAuthProvider {
    type UserInfo = RedditUserInfo;
    const SCOPES: &'static [&'static str] = &[];
    const USER_INFO_URL: &'static str = "https://www.reddit.com/api/v1/me";

    fn make_oauth_client(&self, config: &Config) -> BasicClient {
        let mut redirect_url = config.url();
        redirect_url.set_path("login/reddit_redirect");

        BasicClient::new(
            ClientId::new(config.reddit_client_id.to_owned()),
            Some(ClientSecret::new(config.reddit_client_secret.to_owned())),
            AuthUrl::new(Url::parse("https://www.reddit.com/api/v1/authorize?duration=temporary").unwrap()),
            Some(TokenUrl::new(Url::parse("https://www.reddit.com/api/v1/access_token").unwrap()))
        )
        .set_redirect_url(RedirectUrl::new(redirect_url))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedditUserInfo {
    pub id: String,
    pub username: String,
    pub picture: Option<String>, // url
}

impl UserInfo for RedditUserInfo {
    fn update_existing_player(&self, txn: &Transaction, mod_date: DateTime<Utc>)
        -> Result<Option<PlayerId>, Error> {

        debug!("reddit user info: {:?}", self);

        match txn
            .prepare("SELECT player_id, username, picture FROM player_reddit WHERE id = ?")?
            .query_map(
                params![self.id],
                |row| -> Result<(PlayerId, String, Option<String>), _> {
                    Ok((row.get("player_id")?,
                        row.get("name")?,
                        row.get("picture")?,
                    ))
                }
            )?
            .next() {

            None => Ok(None),
            Some(Ok((player_id, username, picture))) => {
                if username != self.username || picture != self.picture {
                    txn.execute("
                            UPDATE player_reddit
                            SET username = ?, picture = ?, mod_date = ?
                            WHERE user_id = ?
                        ",
                                params![self.username, self.picture, mod_date, self.id])?;
                }
                Ok(Some(player_id))
            },
            Some(Err(e)) => Err(e),
        }

    }

    fn insert_into_db(&self, txn: &Transaction, mod_date: DateTime<Utc>, player_id: PlayerId)
        -> Result<usize, rusqlite::Error> {
        txn.execute("
            INSERT INTO player_reddit (player_id, id, username, picture, mod_date)
            VALUES (?, ?, ?, ?, ?)",
        params![player_id, self.id, self.username, self.picture, mod_date])
    }

    fn name_suggestion(&self) -> String {
        self.username.to_owned()
    }
}
