
use crate::data::DbConn;
use rusqlite::{params, Connection, NO_PARAMS};
use slog::Logger;
use std::path::Path;
use std::sync::Mutex;
use chrono::{DateTime, Utc};

pub fn init_database(log: &Logger, path: &Path) -> DbConn {
    debug!(log, "initializing db at {:?}", path);
    let conn = Connection::open(path).expect("sqlite db");

    conn.execute("
        CREATE TABLE IF NOT EXISTS basho (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            start_date      TEXT NOT NULL,
            venue           TEXT NOT NULL
        )", NO_PARAMS)
        .expect("create basho table");

    conn.execute("
        CREATE TABLE IF NOT EXISTS rikishi (
            id              INTEGER PRIMARY KEY AUTOINCREMENT
        )", NO_PARAMS)
        .expect("create rikishi table");

    conn.execute("
        CREATE TABLE IF NOT EXISTS rikishi_basho (
            rikishi_id      INTEGER NOT NULL REFERENCES rikishi(id),
            basho_id        INTEGER NOT NULL REFERENCES basho(id),
            family_name     TEXT NOT NULL,
            given_name      TEXT NOT NULL,
            rank            TEXT NOT NULL,

            PRIMARY KEY (rikishi_id, basho_id)
        )", NO_PARAMS)
        .expect("create rikishi_basho table");

    conn.execute("
        CREATE TABLE IF NOT EXISTS torikumi (
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
        CREATE TABLE IF NOT EXISTS player (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            join_date       TEXT NOT NULL,
            name            TEXT NOT NULL
        )", NO_PARAMS)
        .expect("create player table");

    conn.execute("
        CREATE TABLE IF NOT EXISTS pick (
            player_id       INTEGER NOT NULL,
            basho_id        INTEGER NOT NULL,
            rikishi_id      INTEGER NOT NULL,

            PRIMARY KEY (player_id, basho_id, rikishi_id)
        )", NO_PARAMS)
        .expect("create pick table");

    conn.execute("CREATE INDEX IF NOT EXISTS basho_id ON pick (basho_id)", NO_PARAMS)
        .expect("create pick.basho_id index");

    let now = Utc::now();
    conn.execute("INSERT INTO player (join_date, name) VALUES ($1, $2)",
            params![now, "Kachi Clasher"])
        .expect("insert single entry into entries table");

    Mutex::new(conn)
}
