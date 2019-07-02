#[macro_use]
extern crate envconfig_derive;
extern crate envconfig;

use rusqlite::{params, Connection, NO_PARAMS};
use std::path::Path;
use chrono::Utc;
use std::path::PathBuf;
use envconfig::Envconfig;

#[derive(Envconfig)]
#[derive(Clone)]
pub struct Config {
    #[envconfig(from = "KACHI_ENV", default = "dev")]
    pub env: String,

    #[envconfig(from = "KACHI_DB_PATH", default = "var/kachiclash.sqlite")]
    pub db_path: PathBuf,
}


fn init_database(path: &Path) {
    println!("initializing db at {:?}", path);
    let conn = Connection::open(path).expect("sqlite db");

    // id is yearmonth
    conn.execute("
        CREATE TABLE basho (
            id              INTEGER PRIMARY KEY,
            start_date      TEXT NOT NULL,
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
            side            TEXT NOT NULL,
            rikishi_id      INTEGER NOT NULL,
            win             INTEGER,

            PRIMARY KEY (basho_id, day, seq, side),
            FOREIGN KEY (rikishi_id, basho_id) REFERENCES rikishi_basho(rikishi_id, basho_id)
        )", NO_PARAMS)
        .expect("create torikumi table");

    conn.execute("
        CREATE TABLE player (
            id              INTEGER PRIMARY KEY AUTOINCREMENT,
            join_date       TEXT NOT NULL,
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

    let now = Utc::now();
    conn.execute("INSERT INTO player (join_date, name) VALUES ($1, $2)",
            params![now, "Kachi Clasher"])
        .expect("insert single entry into entries table");
}

fn main() {
    let config = Config::init().expect("Could not read config from environment");
    init_database(&config.db_path);
}
