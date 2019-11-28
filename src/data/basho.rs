use std::str::FromStr;
use std::convert::From;
use std::collections::{HashMap, HashSet};
use std::fmt;
use std::cmp::max;
use rusqlite::{Connection, NO_PARAMS, Result as SqlResult};
use rusqlite::types::{ToSql, ToSqlOutput, ValueRef, FromSql, FromSqlResult};
use chrono::naive::{NaiveDate, NaiveDateTime};
use chrono::offset::Utc;
use chrono::{DateTime, Datelike};
use serde::{Deserialize, Deserializer};
use itertools::Itertools;

use super::{DataError, PlayerId, Player, RikishiId, Rank, RankGroup, RankSide, Day, Award};

pub struct BashoInfo {
    pub id: BashoId,
    pub start_date: DateTime<Utc>,
    pub venue: String,
    pub external_link: Option<String>,
    pub player_count: u32,
    pub winners: Vec<Player>,
}

impl BashoInfo {
    pub fn with_id(db: &Connection, id: BashoId) -> Result<Option<BashoInfo>, DataError> {
        db.query_row("
            SELECT
                COUNT(*) AS n,
                basho.start_date,
                basho.venue,
                basho.external_link,
                COUNT(DISTINCT pick.player_id) AS player_count
            FROM basho
            LEFT JOIN pick ON pick.basho_id = basho.id
            WHERE basho.id = ?",
                     params![id],
                     |row| {
                         if row.get::<_, u32>("n")? == 0 {
                             Ok(None)
                         } else {
                             Ok(Some(BashoInfo {
                                 id,
                                 start_date: row.get("start_date")?,
                                 venue: row.get("venue")?,
                                 external_link: row.get("external_link")?,
                                 player_count: row.get("player_count")?,
                                 winners: BashoInfo::fetch_basho_winners(&db, id)?,
                             }))
                         }
                     })
            .map_err(|e| e.into())
    }

    pub fn list_all(db: &Connection) -> Result<Vec<BashoInfo>, DataError> {
        let mut winners = BashoInfo::fetch_all_winners(&db)?;
        db.prepare("
                SELECT
                    basho.id,
                    basho.start_date,
                    basho.venue,
                    basho.external_link,
                    COUNT(DISTINCT pick.player_id) AS player_count
                FROM basho
                LEFT JOIN pick ON pick.basho_id = basho.id
                GROUP BY basho.id
                ORDER BY basho.id DESC")?
            .query_map(
                NO_PARAMS,
                |row| {
                    let basho_id = row.get("id")?;
                    Ok(BashoInfo {
                        id: basho_id,
                        start_date: row.get("start_date")?,
                        venue: row.get("venue")?,
                        external_link: row.get("external_link")?,
                        player_count: row.get("player_count")?,
                        winners: winners.remove(&basho_id).unwrap_or_else(|| vec![]),
                    })
                })?
            .collect::<SqlResult<_>>()
            .map_err(|e| e.into())
    }

    pub fn has_started(&self) -> bool {
        self.start_date < Utc::now()
    }

    pub fn link_url(&self) -> String {
        if let Some(str) = &self.external_link {
            str.to_owned()
        } else {
            self.id.url_path()
        }
    }

    fn fetch_basho_winners(db: &Connection, basho_id: BashoId) -> SqlResult<Vec<Player>> {
        Ok(db.prepare("
                SELECT p.*
                FROM award AS a
                JOIN player_info AS p ON p.id = a.player_id
                WHERE a.basho_id = ? AND a.type = ?
            ").unwrap()
            .query_map(params![basho_id, Award::EmperorsCup], |row| Player::from_row(row))?
            .map(|r| r.unwrap())
            .collect())
    }

    fn fetch_all_winners(db: &Connection) -> SqlResult<HashMap<BashoId, Vec<Player>>> {
        let mut map = HashMap::new();
        let mut stmt = db.prepare("
                SELECT
                    a.basho_id,
                    p.*
                FROM award AS a
                JOIN player_info AS p ON p.id = a.player_id
                WHERE a.type = ?
            ")?;
        let rows = stmt.query_map(params![Award::EmperorsCup], |row| {
                Ok((row.get::<_, BashoId>("basho_id")?, Player::from_row(row)?))
            })?;
        for res in rows {
            let (basho_id, player) = res?;
            let vec = map.entry(basho_id).or_insert_with(Vec::new);
            vec.push(player);
        }
        Ok(map)
    }
}



#[derive(Debug, PartialEq, PartialOrd, Eq, Ord, Copy, Clone, Hash)]
pub struct BashoId {
    pub year: i32,
    pub month: u8,
}

impl BashoId {
    pub fn id(self) -> String {
        format!("{:04}{:02}", self.year, self.month)
    }

    pub fn url_path(self) -> String {
        format!("/basho/{}", self.id())
    }

    pub fn season(self) -> String {
        match self.month {
            1 => "Hatsu".to_string(),
            3 => "Haru".to_string(),
            5 => "Natsu".to_string(),
            7 => "Nagoya".to_string(),
            9 => "Aki".to_string(),
            11 => "Kyushu".to_string(),
            _ => self.month_name()
        }
    }

    fn month_name(self) -> String {
        let date = NaiveDate::from_ymd(self.year, self.month.into(), 1);
        format!("{}", date.format("%B"))
    }

    pub fn next_honbasho(self) -> BashoId {
        let next_month = self.month + 2;
        if next_month > 12 {
            BashoId {
                year: self.year + 1,
                month: 1,
            }
        } else {
            BashoId {
                year: self.year,
                month: next_month,
            }
        }
    }
}

impl fmt::Display for BashoId {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        write!(f, "{} – {} {:04}", self.season(), self.month_name(), self.year)
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
    fn deserialize<D>(deserializer: D)
        -> Result<Self, D::Error> where D: Deserializer<'de> {

        let s = String::deserialize(deserializer)?;
        s.parse().map_err(serde::de::Error::custom)
    }
}

impl FromSql for BashoId {
    fn column_result(value: ValueRef) -> FromSqlResult<Self> {
        value
            .as_i64()
            .and_then(|num| {
                Ok(Self {
                    year: (num / 100) as i32,
                    month: (num % 100) as u8,
                })
            })
    }
}

impl ToSql for BashoId {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        let id: u32 = self.id()
            .parse()
            .map_err(|e| rusqlite::Error::ToSqlConversionFailure(Box::new(e)))?;
        Ok(ToSqlOutput::from(id))
    }
}

pub fn save_player_picks(db: &mut Connection, player_id: PlayerId, basho_id: BashoId, picks: [Option<RikishiId>; 5]) -> Result<(), DataError> {
    let txn = db.transaction()?;
    let start_date: DateTime<Utc> = txn.query_row("
        SELECT start_date
        FROM basho
        WHERE id = ?",
        params![basho_id],
        |row| row.get(0))?;
    if start_date < Utc::now() {
        return Err(DataError::BashoHasStarted);
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
        return Err(DataError::InvalidPicks)
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


pub fn update_basho(db: &mut Connection, basho_id: BashoId, venue: &str, start_date: &NaiveDateTime, banzuke: &[(String, Rank)]) -> Result<BashoId, DataError> {
    let txn = db.transaction()?;
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
            })?
        // force evaluation of mapping function and collapse errors into one Result
        .collect::<Result<(), rusqlite::Error>>()
        .map_err(DataError::from)?;
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

pub fn update_torikumi(db: &mut Connection, basho_id: BashoId, day: Day, torikumi: &[TorikumiMatchUpdateData]) -> Result<(), DataError> {

    debug!("updating torikumi for {} day {}", basho_id, day);

    let txn = db.transaction()?;

    let mut rikishi_ids = HashMap::new();
    let mut rikishi_ranks = HashMap::new();
    let mut ambiguous_shikona = Vec::<String>::new();
    txn.prepare("
            SELECT b.rikishi_id, b.family_name, b.rank
            FROM banzuke AS b
            WHERE b.basho_id = ?
        ")?
        .query_map(
            params![basho_id],
            |row| {
                let id: i64 = row.get("rikishi_id")?;
                let family_name: String = row.get("family_name")?;
                let rank: Rank = row.get("rank")?;
                debug!("found mapping {} to rikishi id {}", family_name, id);
                if rikishi_ids.get(&family_name).is_some() {
                    ambiguous_shikona.push(family_name.to_owned());
                }
                rikishi_ids.insert(family_name, id);
                rikishi_ranks.insert(id, rank);
                Ok(())
            })?
        // force evaluation of mapping function and collapse errors into one Result
        .collect::<Result<(), rusqlite::Error>>()
        .map_err(DataError::from)?;
    if !ambiguous_shikona.is_empty() {
        return Err(DataError::AmbiguousShikona {family_names: ambiguous_shikona});
    }

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
                    VALUES (?, ?, ?, ?, ?, ?)
                    ON CONFLICT (basho_id, day, seq, side) DO UPDATE SET
                        rikishi_id = excluded.rikishi_id,
                        win = excluded.win
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


#[derive(Clone)]
pub struct BashoRikishi {
    pub id: RikishiId,
    pub name: String,
    pub rank: Rank,
    pub results: [Option<bool>; 15],
    pub wins: u8,
    pub losses: u8,
    pub picks: u16,
    pub is_player_pick: bool,
}

impl BashoRikishi {
    fn next_day(&self) -> u8 {
        for (i, win) in self.results.iter().enumerate().rev() {
            if win.is_some() {
                return (i + 2) as u8;
            }
        }
        1
    }
}

pub struct BashoRikishiByRank {
    pub rank: String,
    pub rank_group: RankGroup,
    pub east: Option<BashoRikishi>,
    pub west: Option<BashoRikishi>,
}

impl BashoRikishiByRank {
    pub fn next_day(&self) -> u8 {
        max(self.east.as_ref().map_or(1, |r| r.next_day()),
            self.west.as_ref().map_or(1, |r| r.next_day()))
    }
}

pub struct FetchBashoRikishi {
    pub by_rank: Vec<BashoRikishiByRank>,
    pub by_id: HashMap<RikishiId, BashoRikishi>,
}

impl FetchBashoRikishi {
    pub fn with_db(db: &Connection, basho_id: BashoId, picks: &HashSet<RikishiId>)
                     -> SqlResult<Self> {
        debug!("fetching rikishi results for basho {}", basho_id);
        struct FetchedRikishiRow(Rank, RikishiId, String, Option<Day>, Option<bool>, u16);
        let vec: Vec<BashoRikishiByRank> = db.prepare("
            SELECT
                banzuke.rank,
                banzuke.rikishi_id,
                banzuke.family_name,
                torikumi.day,
                torikumi.win,
                (
                    SELECT COUNT(DISTINCT player_id)
                    FROM pick AS p
                    WHERE
                        p.rikishi_id = banzuke.rikishi_id
                        AND p.basho_id = banzuke.basho_id
                ) AS picks
            FROM banzuke
            LEFT NATURAL JOIN torikumi
            WHERE
                banzuke.basho_id = ?
            ORDER BY banzuke.rank DESC, banzuke.rikishi_id, torikumi.day
        ").unwrap()
            .query_map(
                params![basho_id],
                |row| -> SqlResult<FetchedRikishiRow> {
                    Ok(FetchedRikishiRow(
                        row.get("rank")?,
                        row.get("rikishi_id")?,
                        row.get("family_name")?,
                        row.get("day")?,
                        row.get("win")?,
                        row.get("picks")?,
                    ))
                }
            )?
            .collect::<SqlResult<Vec<FetchedRikishiRow>>>()?
            .into_iter()
            .filter(|row| row.0.is_makuuchi())
            .group_by(|row| (row.0.name, row.0.number)) // rank name and number but group east/west together
            .into_iter()
            .sorted_by(|(rank1, _), (rank2, _)| rank1.cmp(rank2))
            .map(|(rank, pair)| {
                let mut out = BashoRikishiByRank {
                    rank: format!("{}{}", rank.0, rank.1),
                    rank_group: RankGroup::for_rank(rank.0, rank.1),
                    east: None,
                    west: None,
                };
                for (_, rows) in &pair.group_by(|row| row.0) {
                    let mut rows = rows.peekable();
                    let arow = rows.peek().unwrap();
                    let side = arow.0.side;
                    let mut rikishi = BashoRikishi {
                        id: arow.1,
                        name: arow.2.to_string(),
                        rank: arow.0,
                        results: [None; 15],
                        wins: 0,
                        losses: 0,
                        picks: arow.5,
                        is_player_pick: picks.contains(&arow.1),
                    };
                    for FetchedRikishiRow(_, _, _, day, win, _) in rows {
                        match win {
                            Some(true) => rikishi.wins += 1,
                            Some(false) => rikishi.losses += 1,
                            None => ()
                        }
                        if let Some(day) = day {
                            rikishi.results[day as usize - 1] = win
                        }
                    }
                    match side {
                        RankSide::East => out.east = Some(rikishi),
                        RankSide::West => out.west = Some(rikishi),
                    }
                }
                out
            })
            .collect();
        let mut map = HashMap::with_capacity(2 * vec.len());
        for brr in &vec {
            if let Some(r) = &brr.east {
                // TODO: how can I specify lifetimes so the HashMap has references to values owned by the Vec to avoid cloning here?
                map.insert(r.id, r.clone());
            }
            if let Some(r) = &brr.west {
                map.insert(r.id, r.clone());
            }
        }
        Ok(Self {
            by_rank: vec,
            by_id: map,
        })
    }
}