use chrono::{DateTime, Utc};
use derive_more::{Deref, Display, From};
use juniper::{GraphQLObject, GraphQLScalar};
use rusqlite::types::{FromSql, FromSqlError, FromSqlResult, ToSqlOutput, ValueRef};
use rusqlite::{
    Connection, Error as SqlError, ErrorCode, OptionalExtension, Result as SqlResult, Row, ToSql,
    Transaction,
};

use super::{Award, BashoId, Heya, Rank, Result};
use crate::data::DataError;
use crate::external::discord::DiscordAuthProvider;
use crate::external::google::GoogleAuthProvider;
use crate::external::reddit::RedditAuthProvider;
use crate::external::{discord, AuthProvider, ImageSize, UserInfo};
use askama::Template;
use rand::random;
use regex::{Regex, RegexBuilder};
use std::collections::HashMap;
use std::ops::RangeInclusive;
use std::sync::LazyLock;
use url::Url;

#[derive(
    Copy,
    Clone,
    Debug,
    Display,
    Deref,
    From,
    PartialEq,
    Eq,
    GraphQLScalar,
    Serialize,
    Deserialize,
    Hash,
)]
#[graphql(transparent)]
#[serde(transparent)]
pub struct PlayerId(i32);

impl TryFrom<i64> for PlayerId {
    type Error = rusqlite::Error;

    fn try_from(value: i64) -> SqlResult<Self> {
        Ok(PlayerId(value.try_into().map_err(|_| {
            rusqlite::Error::IntegralValueOutOfRange(0, value)
        })?))
    }
}

impl ToSql for PlayerId {
    fn to_sql(&self) -> SqlResult<ToSqlOutput> {
        self.0.to_sql()
    }
}

impl FromSql for PlayerId {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let id = value.as_i64()?;
        id.try_into()
            .map_err(|_| FromSqlError::OutOfRange(id))
            .map(PlayerId)
    }
}

pub const NAME_LENGTH: RangeInclusive<usize> = 3..=14;
pub const NAME_REGEX: &str = "^[a-zA-Z][a-zA-Z0-9]*$";

// Because askama makes it tricky to use a {% let player = foo.player %} and then an {% include "player_listing.html" %} to render a standardized player listing subtemplate, we set this up directly as an unescaped template that can be rendered into a parent template like {{foo.player.render().unwrap()|safe}}
#[derive(Debug, Template, GraphQLObject)]
#[template(path = "player_listing.html")]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub join_date: DateTime<Utc>,
    pub emperors_cups: i32,
    pub rank: Option<Rank>,
    admin_level: i32,
    discord_user_id: Option<String>,
    discord_avatar: Option<String>,
    discord_discriminator: Option<String>,
    google_picture: Option<String>,
    reddit_icon: Option<String>,
    pub heyas: Option<Vec<Heya>>, // only populated for own player
}

impl Player {
    pub fn name_is_valid(name: &str) -> bool {
        static RE: LazyLock<Regex> =
            LazyLock::new(|| RegexBuilder::new(NAME_REGEX).build().unwrap());

        NAME_LENGTH.contains(&name.len()) && RE.is_match(name)
    }

    pub fn set_name(txn: &Transaction, player_id: PlayerId, name: &str) -> Result<()> {
        txn.execute(
            "UPDATE player SET name = ? WHERE id = ?",
            params![name, player_id],
        )?;
        Ok(())
    }

    pub fn with_id(
        db: &Connection,
        player_id: PlayerId,
        rank_for_basho: BashoId,
    ) -> Result<Option<Self>> {
        db.query_row(
            "
                SELECT *
                FROM player_info AS p
                LEFT JOIN player_rank AS pr ON pr.player_id = p.id AND pr.before_basho_id = ?
                WHERE p.id = ?
            ",
            params![rank_for_basho, player_id],
            |row| Player::from_row_with_heyas(db, row),
        )
        .optional()
        .map_err(|e| e.into())
    }

    pub fn with_name(
        db: &Connection,
        name: String,
        rank_for_basho: BashoId,
    ) -> Result<Option<Self>> {
        db.query_row(
            "
                SELECT *
                FROM player_info AS p
                LEFT JOIN player_rank AS pr ON pr.player_id = p.id AND pr.before_basho_id = ?
                WHERE p.name = ?
            ",
            params![rank_for_basho, name],
            |row| Player::from_row_with_heyas(db, row),
        )
        .optional()
        .map_err(|e| e.into())
    }

    pub fn list_all(db: &Connection, basho_id: BashoId) -> Result<Vec<Self>> {
        db.prepare(
            "
                SELECT *
                FROM player_info AS p
                LEFT JOIN player_rank AS pr ON pr.player_id = p.id AND pr.before_basho_id = ?
            ",
        )
        .unwrap()
        .query_map(params![basho_id], Player::from_row)
        .map(|mapped_rows| mapped_rows.map(|r| r.unwrap()).collect::<Vec<Player>>())
        .map_err(|e| e.into())
    }

    fn from_row_with_heyas(db: &Connection, row: &Row) -> SqlResult<Self> {
        let mut player = Self::from_row(row)?;
        player.heyas = Some(Heya::for_player(db, player.id)?);
        Ok(player)
    }

    pub fn from_row(row: &Row) -> SqlResult<Self> {
        Ok(Player {
            id: row.get::<_, i32>("id")?.into(),
            name: row.get("name")?,
            join_date: row.get("join_date")?,
            emperors_cups: row.get("emperors_cups")?,
            admin_level: row.get("admin_level")?,
            rank: match row.get("rank") {
                Ok(rank) => rank,
                Err(SqlError::InvalidColumnName(_)) => None,
                Err(e) => return Err(e),
            },
            discord_user_id: row.get("discord_user_id")?,
            discord_avatar: row.get("discord_avatar")?,
            discord_discriminator: row.get("discord_discriminator")?,
            google_picture: row.get("google_picture")?,
            reddit_icon: row.get("reddit_icon")?,
            heyas: None,
        })
    }

    pub fn has_emperors_cup(&self) -> bool {
        self.emperors_cups > 0
    }

    pub fn is_admin(&self) -> bool {
        self.admin_level > 0
    }

    pub fn tiny_thumb(&self) -> String {
        self.image_url(ImageSize::Tiny)
    }

    pub fn medium_thumb(&self) -> String {
        self.image_url(ImageSize::Medium)
    }

    fn image_url(&self, size: ImageSize) -> String {
        const DEFAULT: &str = "/static/img/oicho-silhouette.png";

        if let Some(user_id) = &self.discord_user_id {
            discord::avatar_url(
                user_id,
                &self.discord_avatar,
                self.discord_discriminator
                    .as_ref()
                    .unwrap_or(&"0".to_string()),
                discord::ImageExt::Png,
                size,
            )
            .to_string()
        } else if let Some(icon) = &self.reddit_icon {
            // It's unclear why, but reddit html-escapes the icon_img value in its api return value so we need to unescape it here. In practice, only &amp; appears in the URL so I'm doing a simple replacement.
            Url::parse(&icon.replace("&amp;", "&"))
                .map(|url| url.to_string())
                .unwrap_or_else(|_| DEFAULT.to_owned())
        } else if let Some(picture) = &self.google_picture {
            Url::parse(picture)
                .map(|url| url.to_string())
                .unwrap_or_else(|_| DEFAULT.to_owned())
        } else {
            DEFAULT.to_owned()
        }
    }

    pub fn url_path_for_name(name: &str) -> String {
        format!("/player/{}", name)
    }

    pub fn url_path(&self) -> String {
        Self::url_path_for_name(&self.name)
    }

    pub fn login_service_name(&self) -> &'static str {
        self.login_service_provider()
            .map_or("unknown", |p| p.service_name())
    }

    fn login_service_provider(&self) -> Result<Box<dyn AuthProvider>> {
        if self.discord_user_id.is_some() {
            Ok(Box::new(DiscordAuthProvider))
        } else if self.google_picture.is_some() {
            Ok(Box::new(GoogleAuthProvider))
        } else if self.reddit_icon.is_some() {
            Ok(Box::new(RedditAuthProvider))
        } else {
            Err(DataError::UnknownLoginProvider)
        }
    }

    //     pub async fn update_image(&self, _db: &mut Connection) -> Result<()> {
    //         let _auth = self.login_service_provider()?;
    //
    //         Ok(())
    //     }
}

pub fn player_id_with_external_user(
    db: &mut Connection,
    user_info: Box<dyn UserInfo>,
) -> SqlResult<(PlayerId, bool)> {
    let txn = db.transaction()?;
    let now = Utc::now();
    let existing_player = user_info.update_existing_player(&txn, now)?;
    match existing_player {
        None => {
            let name_suggestion = match user_info.name_suggestion() {
                None => user_info.anon_name_suggestion(),
                Some(name) => {
                    let mut name = name.replace([' ', '_'], "");
                    name.truncate(*NAME_LENGTH.end());
                    if Player::name_is_valid(&name) {
                        name
                    } else {
                        user_info.anon_name_suggestion()
                    }
                }
            };
            let mut name_suffix = "".to_string();
            loop {
                let name = format!("{}{}", name_suggestion, name_suffix);
                match txn.execute(
                    "INSERT INTO player (join_date, name) VALUES (?, ?)",
                    params![now, name],
                ) {
                    Err(SqlError::SqliteFailure(
                        rusqlite::ffi::Error {
                            code: ErrorCode::ConstraintViolation,
                            ..
                        },
                        Some(ref str),
                    )) if str.contains("player.name") => {
                        name_suffix = random::<u16>().to_string();
                        continue;
                    }
                    Err(e) => return Err(e),
                    Ok(_) => break,
                }
            }
            let player_id = txn.last_insert_rowid().try_into()?;
            user_info.insert_into_db(&txn, now, player_id)?;
            txn.commit()?;
            Ok((player_id, true))
        }
        Some(player_id) => {
            txn.commit()?;
            Ok((player_id, false))
        }
    }
}

#[derive(Debug)]
pub struct BashoScore {
    pub basho_id: BashoId,
    pub rank: Option<Rank>,
    pub rikishi: [Option<PlayerBashoRikishi>; 5],
    pub wins: Option<u8>,
    pub place: Option<u16>,
    pub awards: Vec<Award>,
}

impl BashoScore {
    pub fn with_player_id(
        db: &Connection,
        player_id: PlayerId,
        player_name: &str,
    ) -> Result<Vec<Self>> {
        // Build mapping of basho_id => PlayerBashoRikishi that can be inserted into the BashoScores later
        let mut basho_rikishi = HashMap::new();
        {
            struct RikishiRow(BashoId, String, Rank, u8, u8);
            let mut stmt = db
                .prepare(
                    "
                    SELECT
                        b.basho_id,
                        b.rikishi_id,
                        b.family_name,
                        b.rank,
                        COALESCE(SUM(t.win = 1), 0) AS wins,
                        COALESCE(SUM(t.win = 0), 0) AS losses
                    FROM pick AS p
                    JOIN banzuke AS b
                        ON b.basho_id = p.basho_id
                        AND b.rikishi_id = p.rikishi_id
                    LEFT NATURAL JOIN torikumi AS t
                    WHERE p.player_id = ?
                    GROUP BY b.basho_id, b.rikishi_id
                ",
                )
                .unwrap();
            let rikishi_rows = stmt.query_map(params![player_id], |row| {
                Ok(RikishiRow(
                    row.get("basho_id")?,
                    row.get("family_name")?,
                    row.get("rank")?,
                    row.get("wins")?,
                    row.get("losses")?,
                ))
            })?;
            for rr in rikishi_rows {
                let rr = rr?;
                let picks = basho_rikishi
                    .entry(rr.0)
                    .or_insert_with(|| [None, None, None, None, None]);
                picks[rr.2.group().as_index()] = Some(PlayerBashoRikishi {
                    name: rr.1,
                    wins: rr.3,
                    losses: rr.4,
                });
            }
        }

        db.prepare(
            "
                SELECT
                    b.id AS basho_id,
                    pr.rank,
                    COALESCE(r.wins, e.wins) AS wins,
                    COALESCE(r.rank, e.rank) AS place,
                    (
                        SELECT COALESCE(GROUP_CONCAT(a.type), '')
                        FROM award AS a
                        WHERE a.basho_id = b.id AND a.player_id = ?
                    ) AS awards
                FROM basho AS b
                LEFT JOIN player_rank AS pr ON pr.before_basho_id = b.id AND pr.player_id = ?
                LEFT JOIN basho_result AS r ON r.basho_id = b.id AND r.player_id = ?
                LEFT JOIN external_basho_player AS e ON e.basho_id = b.id AND e.name = ?
                ORDER BY b.id DESC
            ",
        )
        .unwrap()
        .query_map(
            params![player_id, player_id, player_id, player_name],
            |row| -> SqlResult<Self> {
                let basho_id = row.get("basho_id")?;
                Ok(BashoScore {
                    basho_id,
                    rank: row.get("rank")?,
                    rikishi: basho_rikishi.remove(&basho_id).unwrap_or_default(),
                    wins: row.get("wins")?,
                    place: row.get("place")?,
                    awards: Award::parse_list(row.get("awards")?),
                })
            },
        )?
        .collect::<SqlResult<_>>()
        .map_err(|e| e.into())
    }
}

#[derive(Debug, serde::Serialize)]
pub struct PlayerBashoRikishi {
    pub name: String,
    pub wins: u8,
    pub losses: u8,
}
