extern crate chrono;

use std::path::Path;
use std::sync::Mutex;
//use rusqlite::types::{FromSql, FromSqlResult, ValueRef};
use rusqlite::{Connection, NO_PARAMS};
use chrono::{DateTime, Utc};

mod rank;
pub use rank::{Rank, RankName, RankSide};

pub type DbConn = Mutex<Connection>;


pub fn make_conn(path: &Path) -> DbConn {
    let conn = Connection::open(path).expect("sqlite db");
    Mutex::new(conn)
}

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
