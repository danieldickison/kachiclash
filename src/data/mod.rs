use std::sync::Mutex;
use std::path::Path;
use rusqlite::{Connection, OpenFlags};

mod rank;
pub use rank::{Rank, RankName, RankSide};

pub mod player;

pub mod basho;
pub use basho::{BashoId};

pub type DbConn = Mutex<Connection>;

pub fn make_conn(path: &Path) -> DbConn {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .expect("sqlite db");
    Mutex::new(conn)
}
