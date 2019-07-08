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


fn main() {
    let config = Config::init().expect("Could not read config from environment");
    init_database(&config.db_path);
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
        CREATE TABLE player_discord (
            player_id   INTEGER NOT NULL,
            user_id     TEXT NOT NULL,
            username    TEXT NOT NULL,
            mod_date    TEXT NOT NULL,

            FOREIGN KEY (player_id) REFERENCES player(id)
        )", NO_PARAMS)
        .expect("create player_discord table");
    conn.execute("CREATE UNIQUE INDEX user_id ON player_discord (user_id)", NO_PARAMS)
        .expect("create player_discord.user_id index");

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

    populate_dummy_data(&conn);
}

fn populate_dummy_data(conn: &Connection) {
    let now = Utc::now();

    let basho_id = 201907;
    conn.execute("INSERT INTO basho (id, start_date, venue) VALUES (?, ?, ?)",
        params![basho_id, now, "Osaka"]).unwrap();

    conn.execute("INSERT INTO rikishi DEFAULT VALUES", NO_PARAMS).unwrap();
    let rikishi1_id = conn.last_insert_rowid();
    conn.execute("INSERT INTO rikishi DEFAULT VALUES", NO_PARAMS).unwrap();
    let rikishi2_id = conn.last_insert_rowid();
    conn.execute("INSERT INTO rikishi DEFAULT VALUES", NO_PARAMS).unwrap();
    let rikishi3_id = conn.last_insert_rowid();
    conn.execute("INSERT INTO rikishi DEFAULT VALUES", NO_PARAMS).unwrap();
    let rikishi4_id = conn.last_insert_rowid();

    conn.execute("INSERT INTO rikishi_basho (rikishi_id, basho_id, family_name, given_name, rank)
        VALUES (?, ?, ?, ?, ?), (?, ?, ?, ?, ?), (?, ?, ?, ?, ?), (?, ?, ?, ?, ?)",
        params![
            rikishi1_id, basho_id, "Hakuho", "Sho", "Y1E",
            rikishi2_id, basho_id, "Kakuryu", "Rikisaburo", "Y1W",
            rikishi3_id, basho_id, "Takakeisho", "Mitsunobu", "O1E",
            rikishi4_id, basho_id, "Mitakeumi", "Hisashi", "S1E"]).unwrap();

    conn.execute("INSERT INTO torikumi (basho_id, day, seq, side, rikishi_id, win)
        VALUES
            (?, 1, 1, 'E', ?, ?), (?, 1, 1, 'W', ?, ?), (?, 1, 2, 'E', ?, ?), (?, 1, 2, 'W', ?, ?),
            (?, 2, 1, 'E', ?, ?), (?, 2, 1, 'W', ?, ?), (?, 2, 2, 'E', ?, ?), (?, 2, 2, 'W', ?, ?)",
        params![
            basho_id, rikishi1_id, 1,
            basho_id, rikishi2_id, 0,
            basho_id, rikishi3_id, 1,
            basho_id, rikishi4_id, 0,
            basho_id, rikishi2_id, 1,
            basho_id, rikishi4_id, 0,
            basho_id, rikishi1_id, 0,
            basho_id, rikishi3_id, 1]).unwrap();

    conn.execute(
        "INSERT INTO player (join_date, name) VALUES (?, ?)",
        params![now, "Daniel"]
    ).unwrap();
    let player_id = conn.last_insert_rowid();

    conn.execute(
        "INSERT INTO pick (player_id, basho_id, rikishi_id) VALUES ($1, $2, $3), ($1, $2, $4)",
        params![player_id, basho_id, rikishi1_id, rikishi4_id]
    ).unwrap();
}

