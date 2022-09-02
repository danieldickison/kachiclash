use rusqlite::config::DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

mod rank;
pub use rank::{Rank, RankGroup, RankName, RankSide};

pub mod player;
pub use player::{Player, PlayerId};

pub mod basho;
pub use basho::{BashoId, BashoInfo, BashoRikishi, BashoRikishiByRank, FetchBashoRikishi};

pub mod award;
pub use award::Award;
use std::error::Error;
use std::fmt;

pub mod leaders;

pub type RikishiId = u32;
pub type Day = u8;

pub type DbConn = Arc<Mutex<Connection>>;

pub fn make_conn(path: &Path) -> DbConn {
    let conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("sqlite db");
    conn.set_db_config(SQLITE_DBCONFIG_ENABLE_FKEY, true)
        .expect("set foreign key enformance to on");
    Arc::new(Mutex::new(conn))
}

type Result<T> = std::result::Result<T, DataError>;

#[derive(Debug)]
pub enum DataError {
    BashoHasStarted,
    InvalidPicks,
    RikishiNotFound { family_name: String },
    AmbiguousShikona { family_names: Vec<String> },
    DatabaseError(rusqlite::Error),
    UnknownLoginProvider,
}

impl From<rusqlite::Error> for DataError {
    fn from(e: rusqlite::Error) -> Self {
        DataError::DatabaseError(e)
    }
}

impl Error for DataError {}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::BashoHasStarted => write!(f, "Basho has already started"),
            DataError::InvalidPicks => write!(f, "Invalid picks"),
            DataError::RikishiNotFound { family_name } => {
                write!(f, "Rikishi not found: {}", family_name)
            }
            DataError::AmbiguousShikona { family_names } => {
                write!(f, "Multiple rikishi with shikona: {:?}", family_names)
            }
            DataError::DatabaseError(e) => write!(f, "Database error: {}", e),
            DataError::UnknownLoginProvider => write!(f, "Unknown login provider"),
        }?;
        Ok(())
    }
}
