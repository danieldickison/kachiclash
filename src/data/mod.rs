extern crate rusqlite;

use std::sync::Mutex;
use rusqlite::types::ToSql;
use rusqlite::{Connection, Error, NO_PARAMS};

pub type DbConn = Mutex<Connection>;

pub fn init_database() -> DbConn {
    // Open a new in-memory SQLite database.
    let conn = Connection::open_in_memory().expect("in memory db");

    conn.execute("CREATE TABLE entries (
                  id              INTEGER PRIMARY KEY,
                  name            TEXT NOT NULL
                  )", NO_PARAMS)
        .expect("create entries table");

    conn.execute("INSERT INTO entries (id, name) VALUES ($1, $2)",
            &[&0, &"Kachi Clasher".to_string() as &ToSql])
        .expect("insert single entry into entries table");

    Mutex::new(conn)
}

pub fn get_name(db_conn: &DbConn) -> Result<String, Error>  {
    db_conn.lock()
        .expect("db connection lock")
        .query_row("SELECT name FROM entries WHERE id = 0",
                   NO_PARAMS, |row| { row.get(0) })
}
