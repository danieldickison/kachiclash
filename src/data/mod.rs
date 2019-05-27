extern crate rusqlite;

use std::sync::Mutex;
use rusqlite::types::{FromSql, FromSqlResult, ValueRef};
use rusqlite::{params, Connection, Error, NO_PARAMS};
use std::time::{SystemTime, Duration, UNIX_EPOCH};

pub type DbConn = Mutex<Connection>;

pub fn init_database() -> DbConn {
    // Open a new in-memory SQLite database.
    let conn = Connection::open_in_memory().expect("in memory db");

    conn.execute("
        CREATE TABLE basho (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            start_date      INTEGER NOT NULL,
            venue           TEXT NOT NULL
        )", NO_PARAMS)
        .expect("create basho table");

    conn.execute("
        CREATE TABLE rikishi (
            id              INTEGER PRIMARY KEY AUTOINCREMENT
        )", NO_PARAMS)
        .expect("create rikishi table");

    conn.execute("
        CREATE TABLE rikishi_basho (
            rikishi_id      INTEGER NOT NULL REFERENCES rikishi(id),
            basho_id        INTEGER NOT NULL REFERENCES basho(id),
            family_name     TEXT NOT NULL,
            given_name      TEXT NOT NULL,
            rank            TEXT NOT NULL,

            PRIMARY KEY (rikishi_id, basho_id)
        )", NO_PARAMS)
        .expect("create rikishi_basho table");

    conn.execute("
        CREATE TABLE torikumi (
            basho_id        INTEGER NOT NULL REFERENCES basho(id),
            day             INTEGER NOT NULL,
            seq             INTEGER NOT NULL,
            east_rikishi_id INTEGER NOT NULL,
            west_rikishi_id INTEGER NOT NULL,
            winner          INTEGER,

            PRIMARY KEY (basho_id, day, seq),
            FOREIGN KEY (east_rikishi_id, basho_id) REFERENCES rikishi_basho(rikishi_id, basho_id),
            FOREIGN KEY (west_rikishi_id, basho_id) REFERENCES rikishi_basho(rikishi_id, basho_id)
        )", NO_PARAMS)
        .expect("create torikumi table");

    conn.execute("
        CREATE TABLE player (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            join_date       INTEGER NOT NULL,
            name            TEXT NOT NULL
        )", NO_PARAMS)
        .expect("create player table");

    conn.execute("
        CREATE TABLE pick (
            player_id       INTEGER NOT NULL,
            basho_id        INTEGER NOT NULL,
            rikishi_id      INTEGER NOT NULL,

            PRIMARY KEY (player_id, basho_id, rikishi_id)
        )", NO_PARAMS)
        .expect("create pick table");

    conn.execute("CREATE INDEX basho_id ON pick (basho_id)", NO_PARAMS)
        .expect("create pick.basho_id index");

    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;
    conn.execute("INSERT INTO player (join_date, name) VALUES ($1, $2)",
            params![now, "Kachi Clasher"])
        .expect("insert single entry into entries table");

    Mutex::new(conn)
}

pub fn get_name(db_conn: &DbConn) -> Result<String, Error>  {
    db_conn.lock()
        .expect("db connection lock")
        .query_row("SELECT name FROM player WHERE id = 1",
                   NO_PARAMS,
                   |row| row.get(0))
}

struct Time(SystemTime);

impl FromSql for Time {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        Ok(Time(SystemTime::UNIX_EPOCH + Duration::from_secs(value.as_i64()? as u64)))
    }
}

#[derive(Debug)]
pub struct Player {
    pub id: i64,
    pub name: String,
    pub join_date: SystemTime
}

pub fn list_players(db_conn: &DbConn) -> Vec<Player> {
    db_conn.lock().unwrap()
        .prepare("SELECT id, name, join_date FROM player").unwrap()
        .query_map(NO_PARAMS, |row| {
            let date: Time = row.get(2)?;
            Ok(Player {
                id: row.get(0)?,
                name: row.get(1)?,
                join_date: date.0
            })
        })
        .and_then(|mapped_rows| {
            Ok(mapped_rows.map(|r| r.unwrap()).collect::<Vec<Player>>())
        }).unwrap()
}
