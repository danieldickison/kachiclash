
use crate::Config;

use std::fmt;
use url::Url;

use oauth2::{
    AuthUrl,
    ClientId,
    ClientSecret,
    RedirectUrl,
    TokenUrl
};
use oauth2::basic::{BasicClient};
use super::{AuthProvider, UserInfo, ImageSize};
use rusqlite::Transaction;
use chrono::{DateTime, Utc};
use crate::data::PlayerId;
use async_trait::async_trait;

const IMG_BASE: &str = "https://cdn.discordapp.com/";

#[derive(Debug)]
pub struct DiscordAuthProvider;

#[async_trait]
impl AuthProvider for DiscordAuthProvider {
    fn service_name(&self) -> &'static str {
        "Discord"
    }

    fn logged_in_user_info_url(&self) -> &'static str {
        "https://discordapp.com/api/v6/users/@me"
    }

    fn oauth_scopes(&self) -> &'static [&'static str] {
        &["identify"]
    }

    fn make_oauth_client(&self, config: &Config) -> BasicClient {
        let mut redirect_url = config.url();
        redirect_url.set_path("login/discord_redirect");

        BasicClient::new(
            ClientId::new(config.discord_client_id.to_owned()),
            Some(ClientSecret::new(config.discord_client_secret.to_owned())),
            AuthUrl::new("https://discordapp.com/api/oauth2/authorize".to_string()).unwrap(),
            Some(TokenUrl::new("https://discordapp.com/api/oauth2/token".to_string()).unwrap())
        )
        .set_redirect_url(RedirectUrl::from_url(redirect_url))
    }

    fn make_user_info_url(&self, user_id: &str) -> String {
        format!("https://discordapp.com/api/v6/users/{}", user_id)
    }

    async fn parse_user_info_response(&self, res: reqwest::Response) -> Result<Box<dyn UserInfo>, failure::Error> {
        Ok(Box::new(res.json::<DiscordUserInfo>().await?))
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct DiscordUserInfo {
    pub id: String,
    pub username: String,
    pub discriminator: String, // 4-digits
    pub avatar: Option<String>,
}

impl UserInfo for DiscordUserInfo {
    fn update_existing_player(&self, txn: &Transaction, mod_date: DateTime<Utc>)
        -> Result<Option<PlayerId>, rusqlite::Error> {

        match txn
            .prepare("SELECT player_id, username, discriminator, avatar FROM player_discord WHERE user_id = ?")?
            .query_map(
                params![self.id],
                |row| -> Result<(PlayerId, String, String, Option<String>), _> {
                    Ok((row.get("player_id")?,
                        row.get("username")?,
                        row.get("discriminator")?,
                        row.get("avatar")?,
                    ))
                }
            )?
            .next() {

            None => Ok(None),
            Some(Ok((player_id, username, discriminator, avatar))) => {
                if username != self.username || discriminator != self.discriminator || avatar != self.avatar {
                    txn.execute("
                            UPDATE player_discord
                            SET username = ?, discriminator = ?, avatar = ?, mod_date = ?
                            WHERE user_id = ?
                        ",
                                params![self.username, self.discriminator, self.avatar, mod_date, self.id])?;
                }
                Ok(Some(player_id))
            },
            Some(Err(e)) => Err(e),
        }

    }

    fn insert_into_db(&self, txn: &Transaction, mod_date: DateTime<Utc>, player_id: PlayerId)
        -> Result<usize, rusqlite::Error> {
        txn.execute("
            INSERT INTO player_discord (player_id, user_id, username, discriminator, avatar, mod_date)
            VALUES (?, ?, ?, ?, ?, ?)",
        params![player_id, self.id, self.username, self.discriminator, self.avatar, mod_date])
    }

    fn name_suggestion(&self) -> Option<String> {
        Some(self.username.to_owned())
    }
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

pub fn avatar_url(user_id: &str, avatar: &Option<String>, discriminator: &str, ext: ImageExt, size: ImageSize) -> Url {
    let base = Url::parse(IMG_BASE).unwrap();
    if let Some(hash) = &avatar {
        base.join(&format!("avatars/{}/{}.{}?size={}", user_id, hash, ext, size as i32)[..]).unwrap()
    } else {
        let discrim = u16::from_str_radix(discriminator, 10).unwrap_or(0) % 5;
        base.join(&format!("embed/avatars/{}.png?size={}", discrim, size as i32)[..]).unwrap()
    }
}
