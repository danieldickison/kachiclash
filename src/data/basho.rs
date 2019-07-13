use std::str::FromStr;
use std::convert::From;
use std::fmt;
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef, FromSql, FromSqlResult, FromSqlError};
use chrono::naive::NaiveDate;
use chrono::offset::Utc;
use chrono::{DateTime, Datelike};
use serde::{Deserialize, Deserializer};
use rusqlite::{Connection, OptionalExtension};
use failure::Error;
use itertools::Itertools;

use super::{DataError, PlayerId, RikishiId, Rank, RankGroup};

pub struct BashoInfo {
    pub id: BashoId,
    pub start_date: DateTime<Utc>,
    pub venue: String,
    pub player_count: u32,
}

impl BashoInfo {
    pub fn with_id(db: &Connection, id: BashoId) -> Result<Option<BashoInfo>, Error> {
        db.query_row("
            SELECT
                basho.start_date,
                basho.venue,
                COUNT(DISTINCT pick.player_id) AS player_count
            FROM basho
            LEFT JOIN pick ON pick.basho_id = basho.id
            WHERE basho.id = ?",
            params![id],
            |row| {
                Ok(BashoInfo {
                    id: id,
                    start_date: row.get("start_date")?,
                    venue: row.get("venue")?,
                    player_count: row.get("player_count")?,
                })
            })
            .optional()
            .map_err(|e| e.into())
    }

    pub fn has_started(&self) -> bool {
        self.start_date < Utc::now()
    }
}



#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone)]
pub struct BashoId {
    pub year: i32,
    pub month: u8,
}

impl BashoId {
    pub fn url_path(&self) -> String {
        format!("/basho/{:04}{:02}", self.year, self.month)
    }
}

impl fmt::Display for BashoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let date = NaiveDate::from_ymd(self.year, self.month.into(), 1);
        write!(f, "{}", date.format("%B %Y"))
    }
}

impl FromStr for BashoId {
    type Err = chrono::format::ParseError;
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let with_day = format!("{}01", s);
        NaiveDate::parse_from_str(&with_day, "%Y%m%d").map(|date|
            Self {
                year: date.year(),
                month: date.month() as u8,
            }
        )
    }
}

impl<'de> Deserialize<'de> for BashoId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromSql for BashoId {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value
            .as_str()
            .and_then(|str|
                str.parse().map_err(|err| FromSqlError::Other(Box::new(err)))
            )
    }
}

impl ToSql for BashoId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let id: u32 = format!("{:04}{:02}", self.year, self.month)
            .parse()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(id))
    }
}


pub fn save_player_picks(db: &mut Connection, player_id: PlayerId, basho_id: BashoId, picks: [Option<RikishiId>; 5]) -> Result<(), Error> {
    let txn = db.transaction()?;
    let start_date: DateTime<Utc> = txn.query_row("
        SELECT start_date
        FROM basho
        WHERE id = ?",
        params![basho_id],
        |row| row.get(0))?;
    if start_date < Utc::now() {
        return Err(DataError::BashoHasStarted.into());
    }

    let rank_groups: Vec<RankGroup> = txn.prepare("
        SELECT rank
        FROM rikishi_basho
        WHERE basho_id = ? AND rikishi_id IN (?, ?, ?, ?, ?)")?
    .query_map(params![basho_id, picks[0], picks[1], picks[2], picks[3], picks[4]], |row| row.get(0))?
    .map(|rank: rusqlite::Result<Rank>| rank.unwrap().group())
    .collect();
    debug!("rank groups {:?} for picks {:?}", rank_groups, picks);
    if rank_groups.clone().into_iter().unique().collect::<Vec<RankGroup>>() != rank_groups {
        return Err(DataError::InvalidPicks.into())
    }

    txn.execute("
        DELETE FROM pick
        WHERE player_id = ? AND basho_id = ?",
        params![player_id, basho_id])?;
    for rikishi_id in &picks {
        if let Some(rikishi_id) = rikishi_id {
            debug!("inserting player {} pick {} for {}", player_id, rikishi_id, basho_id);
            txn.execute("
                INSERT INTO pick (player_id, basho_id, rikishi_id)
                VALUES (?, ?, ?)",
                params![player_id, basho_id, rikishi_id])?;
        }
    }
    txn.commit()?;

    Ok(())
}
