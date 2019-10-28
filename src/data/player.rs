use rusqlite::{Row, Connection, OptionalExtension};
use chrono::{DateTime, Utc};

use crate::external::discord;
use super::{Award, DataError};

pub type PlayerId = i64;

#[derive(Debug)]
pub struct Player {
    pub id: PlayerId,
    pub name: String,
    pub join_date: DateTime<Utc>,
    admin_level: u8,
    pub discord_info: Option<discord::UserInfo>,
}

impl Player {
    pub fn with_id(db: &Connection, player_id: PlayerId) -> Result<Option<Self>, DataError> {
        db.query_row("
                SELECT
                    p.id, p.name, p.join_date, p.admin_level,
                    d.user_id, d.username, d.avatar, d.discriminator,
                    COALESCE((
                        SELECT 1
                        FROM award AS a
                        WHERE a.player_id = p.id AND type = ?
                        LIMIT 1
                    ), 0) AS has_emperors_cup
                FROM player AS p
                LEFT JOIN player_discord AS d ON d.player_id = p.id
                WHERE p.id = ?
            ", params![Award::EmperorsCup, player_id], |row| Player::from_row(row))
            .optional()
            .map_err(|e| e.into())
    }

    pub fn list_all(db: &Connection) -> Result<Vec<Self>, DataError> {
        db.prepare("
                SELECT
                    p.id, p.name, p.join_date, p.admin_level,
                    d.user_id, d.username, d.avatar, d.discriminator,
                    COALESCE((
                        SELECT 1
                        FROM award AS a
                        WHERE a.player_id = p.id AND type = ?
                        LIMIT 1
                    ), 0) AS has_emperors_cup
                FROM player AS p
                LEFT JOIN player_discord AS d ON d.player_id = p.id
            ").unwrap()
            .query_map(params![Award::EmperorsCup], |row| Player::from_row(row))
            .and_then(|mapped_rows| {
                Ok(mapped_rows.map(|r| r.unwrap()).collect::<Vec<Player>>())
            })
            .map_err(|e| e.into())
    }

    pub fn from_row(row: &Row) -> Result<Self, rusqlite::Error> {
        let mut name: String = row.get("name")?;
        let has_emperors_cup: bool = row.get("has_emperors_cup")?;
        if has_emperors_cup {
            name.push_str(" ");
            name.push_str(Award::EmperorsCup.emoji());
        }
        Ok(Player {
            id: row.get("id")?,
            name: name,
            join_date: row.get("join_date")?,
            admin_level: row.get("admin_level")?,
            discord_info: match row.get("user_id")? {
                Some(user_id) => Some(discord::UserInfo {
                    id: user_id,
                    username: row.get("username")?,
                    avatar: row.get("avatar")?,
                    discriminator: row.get("discriminator")?,
                }),
                None => None
            }
        })
    }

    pub fn is_admin(&self) -> bool {
        self.admin_level > 0
    }

    pub fn tiny_thumb(&self) -> String {
        match &self.discord_info {
            Some(info) => discord::avatar_url(&info, discord::ImageExt::PNG, discord::ImageSize::TINY).to_string(),
            None => "/static/default_avatar.png".to_string(),
        }
    }
}

pub fn player_id_with_discord_user(db: &mut Connection, user_info: discord::UserInfo) -> Result<PlayerId, rusqlite::Error> {
    let txn = db.transaction()?;
    let now = Utc::now();
    let existing_row = txn
        .prepare("SELECT player_id, username FROM player_discord WHERE user_id = ?")?
        .query_map(
            params![user_info.id],
            |row| -> Result<(i64, String), _> {
                Ok((row.get("player_id")?, row.get("username")?))
            }
        )?
        .next();
    match existing_row {
        None => {
            txn.execute("INSERT INTO player (join_date, name) VALUES (?, ?)",
                        params![now, user_info.username]).unwrap();
            let player_id = txn.last_insert_rowid();
            txn.execute("INSERT INTO player_discord (player_id, user_id, username, avatar, discriminator, mod_date) VALUES (?, ?, ?, ?, ?, ?)",
                        params![player_id, user_info.id, user_info.username, user_info.avatar, user_info.discriminator, now]).unwrap();
            txn.commit()?;
            Ok(player_id)
        },
        Some(Ok((player_id, username))) => {
            if username != user_info.username {
                txn.execute("
                        UPDATE player_discord
                        SET username = ?, mod_date = ?
                        WHERE user_id = ?
                    ",
                            params![user_info.username, now, user_info.id])?;
            }
            txn.commit()?;
            Ok(player_id)
        },
        Some(Err(e)) => Err(e)
    }
}