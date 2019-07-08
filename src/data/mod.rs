extern crate chrono;

use std::path::Path;
use std::sync::Mutex;
//use rusqlite::types::{FromSql, FromSqlResult, ValueRef};
use rusqlite::{Connection};

mod rank;
pub use rank::{Rank, RankName, RankSide};

pub mod player;

pub type DbConn = Mutex<Connection>;


pub fn make_conn(path: &Path) -> DbConn {
    let conn = Connection::open(path).expect("sqlite db");
    Mutex::new(conn)
}
