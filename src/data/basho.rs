use std::str::FromStr;
use std::convert::From;
use std::collections::HashMap;
use std::fmt;
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef, FromSql, FromSqlResult, FromSqlError};
use chrono::naive::{NaiveDate, NaiveDateTime};
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
                    id,
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
    pub fn url_path(self) -> String {
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
        NaiveDate::parse_from_str(&with_day, "%Y%m%d").map(|date| date.into())
    }
}

impl From<NaiveDate> for BashoId {
    fn from(date: NaiveDate) -> Self {
        Self {
            year: date.year(),
            month: date.month() as u8,
        }
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
        FROM banzuke
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


pub fn make_basho(db: &mut Connection, venue: &str, start_date: &NaiveDateTime, banzuke: &[(String, Rank)]) -> Result<BashoId, DataError> {
    let txn = db.transaction()?;
    let basho_id: BashoId = start_date.date().into();
    txn.execute("
        INSERT INTO basho (id, start_date, venue)
        VALUES (?, ?, ?)
        ON CONFLICT (id) DO UPDATE SET
            start_date = excluded.start_date,
            venue = excluded.venue
        ",
        params![basho_id, start_date, venue])?;

    let mut rikishi_ids = HashMap::new();
    let mut given_names = HashMap::new();
    let query_str = format!("
            SELECT id, family_name, given_name
            FROM rikishi
            WHERE family_name IN ({})
        ",
        banzuke.iter().map(|(_, _)| "?").join(", ")
    );
    let mut ambiguous_shikona = Vec::<String>::new();
    txn.prepare(query_str.as_str())?
        .query_map(
            banzuke.iter().map(|(name, _)| name),
            |row| {
                let id: i64 = row.get("id")?;
                let family_name: String = row.get("family_name")?;
                let given_name: String = row.get("given_name")?;
                if rikishi_ids.get(&family_name).is_some() {
                    ambiguous_shikona.push(family_name.to_owned());
                }
                rikishi_ids.insert(family_name, id);
                given_names.insert(id, given_name);
                Ok(())
            })?;
    if !ambiguous_shikona.is_empty() {
        return Err(DataError::AmbiguousShikona {family_names: ambiguous_shikona});
    }

    for (family_name, rank) in banzuke {
        let rikishi_id = match rikishi_ids.get(family_name) {
            Some(id) => id.to_owned(),
            None => {
                txn.execute("
                        INSERT INTO rikishi (family_name, given_name)
                        VALUES (?, ?)
                    ",
                    params![family_name, ""])?; // TODO given_name
                txn.last_insert_rowid()
            }
        };
        let given_name = given_names.get(&rikishi_id).unwrap_or(&"".to_string()).to_owned(); // TODO given_name
        txn.execute("
                INSERT INTO banzuke (rikishi_id, basho_id, family_name, given_name, rank)
                VALUES (?, ?, ?, ?, ?)
                ON CONFLICT (rikishi_id, basho_id) DO UPDATE SET
                    family_name = excluded.family_name,
                    given_name = excluded.given_name,
                    rank = excluded.rank
            ",
            params![rikishi_id, basho_id, family_name, given_name, rank])?;
    }
    txn.commit()?;

    Ok(basho_id)
}

#[derive(Debug, Deserialize)]
pub struct TorikumiMatchUpdateData {
    winner: String,
    loser: String,
}

pub fn update_torikumi(db: &mut Connection, basho_id: &BashoId, day: &u8, torikumi: &Vec<TorikumiMatchUpdateData>) -> Result<(), DataError> {

    debug!("updating torikumi for {} day {}", basho_id, day);
    debug!("basho_id as sql: {:?}", basho_id.to_sql());

    let txn = db.transaction()?;

    let mut rikishi_ids = HashMap::new();
    let mut rikishi_ranks = HashMap::new();
    txn.prepare("
            SELECT b.rikishi_id, b.family_name, b.rank
            FROM banzuke AS b
            WHERE b.basho_id = ?
        ")?
        .query_map(
            params![basho_id],
            |row| {
                debug!("got a row");
                let id: i64 = row.get("id")?;
                let family_name: String = row.get("family_name")?;
                let rank: Rank = row.get("rank")?;
                debug!("found mapping {} to rikishi id {}", family_name, id);
                rikishi_ids.insert(family_name, id);
                rikishi_ranks.insert(id, rank);
                Ok(())
            })?;

    for (seq, TorikumiMatchUpdateData {winner, loser})
        in torikumi.iter().enumerate() {

        let winner_id = rikishi_ids.get(winner)
            .ok_or_else(|| DataError::RikishiNotFound {family_name: winner.to_owned()})?;
        let loser_id = rikishi_ids.get(loser)
            .ok_or_else(|| DataError::RikishiNotFound {family_name: loser.to_owned()})?;
        let winner_rank = rikishi_ranks.get(winner_id).unwrap();
        let loser_rank = rikishi_ranks.get(loser_id).unwrap();

        let insert_1 = |side, rikishi_id, win| {
            txn.execute("
                    INSERT INTO torikumi (basho_id, day, seq, side, rikishi_id, win)
                    VALUES (?, ?, ?, ?, ?)
                    ON CONFLICT (rikishi_id, basho_id) DO UPDATE SET
                        family_name = excluded.family_name,
                        given_name = excluded.given_name,
                        rank = excluded.rank
                ",
                params![basho_id, day, seq as u32, side, rikishi_id, win])
        };

        // Figuring out the side: the rikishi with the higher rank appear on their own rank.side
        insert_1(
            if winner_rank > loser_rank { winner_rank.side } else { loser_rank.side.other() },
            winner_id,
            true
        )?;
        insert_1(
            if loser_rank > winner_rank { loser_rank.side } else { winner_rank.side.other() },
            loser_id,
            false
        )?;
    }

    txn.commit()?;

    Ok(())
}