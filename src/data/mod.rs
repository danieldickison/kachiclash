use heya::HeyaId;
use rusqlite::config::DbConfig::SQLITE_DBCONFIG_ENABLE_FKEY;
use rusqlite::{Connection, OpenFlags};
use std::path::Path;
use std::sync::{Arc, Mutex};

#[cfg(debug_assertions)]
use rusqlite::trace::{TraceEvent, TraceEventCodes};

mod rank;
pub use rank::{Rank, RankDivision, RankGroup, RankName, RankSide};

pub mod player;
pub use player::{Player, PlayerId};

pub mod basho;
pub use basho::{BashoInfo, BashoRikishi, BashoRikishiByRank, FetchBashoRikishi};

pub mod basho_id;
pub use basho_id::BashoId;

pub mod award;
pub use award::Award;
use std::error::Error;
use std::fmt;

pub mod leaders;

pub mod push;

pub mod heya;
pub use heya::Heya;

pub type RikishiId = u32;
pub type Day = u8;

pub type DbConn = Arc<Mutex<Connection>>;

pub fn make_conn(path: &Path) -> DbConn {
    #[allow(unused_mut)]
    let mut conn = Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .expect("sqlite db");
    conn.set_db_config(SQLITE_DBCONFIG_ENABLE_FKEY, true)
        .expect("set foreign key enformance to on");

    #[cfg(debug_assertions)]
    conn.trace_v2(TraceEventCodes::SQLITE_TRACE_PROFILE, Some(db_trace));

    Arc::new(Mutex::new(conn))
}

#[cfg(debug_assertions)]
fn db_trace(event: TraceEvent) {
    use regex::Regex;
    use std::sync::LazyLock;
    static RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    if let TraceEvent::Profile(stmt, duration) = event {
        trace!(
            "sqlite: {} ({:.3}s)",
            RE.replace_all(&stmt.sql(), " "),
            duration.as_secs_f32()
        );
    }
}

type Result<T> = std::result::Result<T, DataError>;

#[derive(Debug)]
pub enum DataError {
    BashoHasStarted,
    InvalidPicks,
    HeyaIntegrity {
        what: String,
    },
    RikishiNotFound {
        family_name: String,
    },
    AmbiguousShikona {
        family_names: Vec<String>,
    },
    HeyaNotFound {
        slug: Option<String>,
        id: Option<HeyaId>,
    },
    DatabaseError(rusqlite::Error),
    WebPushError(web_push::WebPushError),
    JsonError(serde_json::Error),
    UnknownLoginProvider,
}

impl From<rusqlite::Error> for DataError {
    fn from(e: rusqlite::Error) -> Self {
        DataError::DatabaseError(e)
    }
}

impl From<web_push::WebPushError> for DataError {
    fn from(e: web_push::WebPushError) -> Self {
        DataError::WebPushError(e)
    }
}

impl From<serde_json::Error> for DataError {
    fn from(e: serde_json::Error) -> Self {
        DataError::JsonError(e)
    }
}

impl Error for DataError {}

impl fmt::Display for DataError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DataError::BashoHasStarted => write!(f, "Basho has already started"),
            DataError::InvalidPicks => write!(f, "Invalid picks"),
            DataError::HeyaIntegrity { what } => write!(f, "Heya integrity error: {}", what),
            DataError::RikishiNotFound { family_name } => {
                write!(f, "Rikishi not found: {}", family_name)
            }
            DataError::AmbiguousShikona { family_names } => {
                write!(f, "Multiple rikishi with shikona: {:?}", family_names)
            }
            DataError::HeyaNotFound { slug, id } => {
                write!(f, "Heya not found for slug {slug:?} or id {id:?}")
            }
            DataError::DatabaseError(e) => write!(f, "Database error: {}", e),
            DataError::UnknownLoginProvider => write!(f, "Unknown login provider"),
            DataError::WebPushError(e) => write!(f, "Web Push error: {}", e),
            DataError::JsonError(e) => write!(f, "JSON error: {}", e),
        }?;
        Ok(())
    }
}
