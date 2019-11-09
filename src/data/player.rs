use rusqlite::{Row, Connection, OptionalExtension, ErrorCode, Error as SqlError, NO_PARAMS};
use chrono::{DateTime, Utc};

use crate::external::{self, discord};
use super::{DataError};
use rand::random;
use url::Url;
use std::ops::RangeInclusive;
use regex::{Regex, RegexBuilder};

pub type PlayerId = i64;

pub const NAME_LENGTH: RangeInclusive<usize> = (3..=14);
pub const NAME_REGEX: &str = "^[a-zA-Z][a-zA-Z0-9]*$";

#[derive(Debug, Clone)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub join_date: DateTime<Utc>,
    pub emperors_cups: u8,
    admin_level: u8,
    discord_user_id: Option<String>,
    discord_avatar: Option<String>,
    discord_discriminator: Option<String>,
    google_picture: Option<String>,
    reddit_icon: Option<String>,
}

impl Player {
    pub fn with_id(db: &Connection, player_id: PlayerId) -> Result<Option<Self>, DataError> {
        db.query_row("
                SELECT *
                FROM player_info AS p
                WHERE p.id = ?
            ", params![player_id], |row| Player::from_row(row))
            .optional()
            .map_err(|e| e.into())
    }

    pub fn list_all(db: &Connection) -> Result<Vec<Self>, DataError> {
        db.prepare("
                SELECT * FROM player_info
            ").unwrap()
            .query_map(NO_PARAMS, |row| Player::from_row(row))
            .and_then(|mapped_rows| {
                Ok(mapped_rows.map(|r| r.unwrap()).collect::<Vec<Player>>())
            })
            .map_err(|e| e.into())
    }

    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        Ok(Player {
            id: row.get("id")?,
            name: row.get("name")?,
            join_date: row.get("join_date")?,
            emperors_cups: row.get("emperors_cups")?,
            admin_level: row.get("admin_level")?,
            discord_user_id: row.get("discord_user_id")?,
            discord_avatar: row.get("discord_avatar")?,
            discord_discriminator: row.get("discord_discriminator")?,
            google_picture: row.get("google_picture")?,
            reddit_icon: row.get("reddit_icon")?,
        })
    }

    pub fn has_emperors_cup(&self) -> bool {
        self.emperors_cups > 0
    }

    pub fn is_admin(&self) -> bool {
        self.admin_level > 0
    }

    pub fn tiny_thumb(&self) -> String {
        const DEFAULT: &str = "/static/img/oicho-silhouette.png";

        if let Some(user_id) = &self.discord_user_id {
            discord::avatar_url(
                &user_id,
                &self.discord_avatar,
                &self.discord_discriminator.as_ref().unwrap_or(&"0".to_string()),
                discord::ImageExt::PNG,
                discord::ImageSize::TINY).to_string()
        } else if let Some(icon) = &self.reddit_icon {
            // It's unclear why, but reddit html-escapes the icon_img value in its api return value so we need to unescape it here. In practice, only &amp; appears in the URL so I'm doing a simple replacement.
            Url::parse(&icon.replace("&amp;", "&"))
                .map(|url| url.to_string())
                .unwrap_or_else(|_| DEFAULT.to_owned())
        } else if let Some(picture) = &self.google_picture {
            Url::parse(&picture)
                .map(|url| url.to_string())
                .unwrap_or_else(|_| DEFAULT.to_owned())
        } else {
            DEFAULT.to_owned()
        }
    }

    pub fn url_path(&self) -> String {
        format!("/player/{}", self.id)
    }
}

pub fn player_id_with_external_user(db: &mut Connection, user_info: impl external::UserInfo) -> Result<(PlayerId, bool), rusqlite::Error> {
    let txn = db.transaction()?;
    let now = Utc::now();
    let existing_player = user_info.update_existing_player(&txn, now)?;
    match existing_player {
        None => {
            let name_suggestion = match user_info.name_suggestion() {
                None => user_info.anon_name_suggestion(),
                Some(name) => {
                    let mut name = name.replace(" ", "").replace("_", "");
                    name.truncate(*NAME_LENGTH.end());
                    if name_is_valid(&name) {
                        name
                    } else {
                        user_info.anon_name_suggestion()
                    }
                }
            };
            let mut name_suffix = "".to_string();
            loop {

                let name = format!("{}{}", name_suggestion, name_suffix);
                match txn.execute("INSERT INTO player (join_date, name) VALUES (?, ?)",
                                  params![now, name]) {
                    Err(SqlError::SqliteFailure(rusqlite::ffi::Error { code: ErrorCode::ConstraintViolation, .. }, Some(ref str)))
                    if str.contains("player.name") => {
                        name_suffix = random::<u16>().to_string();
                        continue;
                    },
                    Err(e) => return Err(e),
                    Ok(_) => break,
                }
            }
            let player_id = txn.last_insert_rowid();
            user_info.insert_into_db(&txn, now, player_id)?;
            txn.commit()?;
            Ok((player_id, true))
        },
        Some(player_id) => {
            txn.commit()?;
            Ok((player_id, false))
        }
    }
}

pub fn name_is_valid(name: &str) -> bool {
    lazy_static! {
        static ref RE: Regex =
            RegexBuilder::new(NAME_REGEX)
                .build()
                .unwrap();
    }

    NAME_LENGTH.contains(&name.len()) && RE.is_match(&name)
}
