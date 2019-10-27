use std::sync::{Mutex, Arc};
use std::path::Path;
use rusqlite::{Connection, OpenFlags};
use rusqlite::config::DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY;

mod rank;
pub use rank::{Rank, RankName, RankSide, RankGroup};

pub mod player;
pub use player::{PlayerId, Player};

pub mod basho;
pub use basho::{BashoId, BashoInfo};

pub mod award;
pub use award::Award;

pub type RikishiId = u32;
pub type Day = u8;

pub type DbConn = Arc<Mutex<Connection>>;

pub fn make_conn(path: &Path) -> DbConn {
    let conn = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX)
        .expect("sqlite db");
    conn.set_db_config(SQLITE_DBCONFIG_ENABLE_FKEY, true)
        .expect("set foreign key enformance to on");
    Arc::new(Mutex::new(conn))
}

#[derive(Fail, Debug)]
pub enum DataError {
    #[fail(display = "Basho has already started")]
    BashoHasStarted,

    #[fail(display = "Invalid picks")]
    InvalidPicks,

    #[fail(display = "Rikishi not found: {}", family_name)]
    RikishiNotFound {
        family_name: String,
    },

    #[fail(display = "Multiple rikishi with shikona: {:?}", family_names)]
    AmbiguousShikona {
        family_names: Vec<String>,
    },

    #[fail(display = "Database error: {}", _0)]
    DatabaseError(rusqlite::Error),
}

impl From<rusqlite::Error> for DataError {
    fn from(e: rusqlite::Error) -> Self {
        DataError::DatabaseError(e)
    }
}
