extern crate rusqlite;

use std::sync::Mutex;
use rusqlite::types::ToSql;
use rusqlite::{params, Connection, Error, NO_PARAMS};
use std::time::{SystemTime, UNIX_EPOCH};

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
