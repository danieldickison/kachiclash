extern crate chrono;

use std::path::Path;
use std::sync::Mutex;
//use rusqlite::types::{FromSql, FromSqlResult, ValueRef};
use rusqlite::{Connection, Error, NO_PARAMS};
use chrono::{DateTime, Utc};

mod rank;
pub use rank::{Rank, RankName, RankSide};

pub type DbConn = Mutex<Connection>;


pub fn make_conn(path: &Path) -> DbConn {
    let conn = Connection::open(path).expect("sqlite db");
    Mutex::new(conn)
}

pub fn get_name(db_conn: &DbConn) -> Result<String, Error>  {
    db_conn.lock()
        .expect("db connection lock")
        .query_row("SELECT name FROM player WHERE id = 1",
                   NO_PARAMS,
                   |row| row.get(0))
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
