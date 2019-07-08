use rusqlite::{NO_PARAMS};
use chrono::{DateTime, Utc};

use crate::data::DbConn;
use crate::external::discord;

pub type PlayerID = i64;

#[derive(Debug)]
pub struct Player {
    pub id: i64,
    pub name: String,
    pub join_date: DateTime<Utc>
}

pub fn list_players(db_conn: &DbConn) -> Vec<Player> {
    db_conn.lock().unwrap()
        .prepare("SELECT id, name, join_date FROM player").unwrap()
        .query_map(NO_PARAMS, |row| {
            Ok(Player {
                id: row.get(0)?,
                name: row.get(1)?,
                join_date: row.get(2)?
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
                Ok((row.get(0)?, row.get(1)?))
            }
        )?
        .next();
    match existing_row {
        None => {
            txn.execute("INSERT INTO player (join_date, name) VALUES (?, ?)",
                params![now, user_info.username]).unwrap();
            let player_id = txn.last_insert_rowid();
            txn.execute("INSERT INTO player_discord (player_id, user_id, username, mod_date) VALUES (?, ?, ?, ?)",
                params![player_id, user_info.id, user_info.username, now]).unwrap();
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
