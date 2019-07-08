use rusqlite::{NO_PARAMS};
use chrono::{DateTime, Utc};

use crate::data::DbConn;
use crate::external::discord;

pub type PlayerID = i64;

#[derive(Debug)]
pub struct Player {
    pub id: i64,
    pub name: String,
    pub join_date: DateTime<Utc>,
    pub discord_info: Option<discord::UserInfo>,
}

impl Player {
    pub fn small_thumb(&self) -> String {
        match &self.discord_info {
            Some(info) => discord::avatar_url(&info, discord::ImageExt::PNG, discord::ImageSize::SMALL).to_string(),
            None => "/static/default_avatar.png".to_string(),
        }
    }
}

pub fn list_players(db_conn: &DbConn) -> Vec<Player> {
    db_conn.lock().unwrap()
        .prepare("
            SELECT
                p.id, p.name, p.join_date,
                d.user_id, d.username, d.avatar, d.discriminator
            FROM player AS p
            LEFT JOIN player_discord AS d ON d.player_id = p.id
        ").unwrap()
        .query_map(NO_PARAMS, |row| {
            Ok(Player {
                id: row.get("id")?,
                name: row.get("name")?,
                join_date: row.get("join_date")?,
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
        })
        .and_then(|mapped_rows| {
            Ok(mapped_rows.map(|r| r.unwrap()).collect::<Vec<Player>>())
        }).unwrap()
}

pub fn player_for_discord_user(db_conn: &DbConn, user_info: discord::UserInfo) -> Result<PlayerID, rusqlite::Error> {
    let mut conn = db_conn.lock().unwrap();
    let txn = conn.transaction().unwrap();
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
