use chrono::naive::NaiveDateTime;
use chrono::offset::Utc;
use chrono::DateTime;
use itertools::Itertools;
use result::ResultIteratorExt;
use rusqlite::{params_from_iter, Connection, Result as SqlResult, Transaction};
use serde::Deserialize;
use std::cmp::max;
use std::collections::{HashMap, HashSet};
use std::convert::From;

use super::leaders::HistoricLeader;
use super::{
    Award, BashoId, DataError, Day, Player, PlayerId, Rank, RankGroup, RankSide, Result, RikishiId,
};
use crate::data::leaders::{assign_ord, Rankable};

#[derive(Debug)]
pub struct BashoInfo {
    pub id: BashoId,
    pub start_date: DateTime<Utc>,
    pub venue: String,
    pub external_link: Option<String>,
    pub player_count: usize,
    pub winners: Vec<Player>,
    pub winning_score: Option<u8>,
}

const VERY_FIRST_BASHO: &str = "201901";

impl BashoInfo {
    /// Returns the current basho id if one is in session; otherwise returns the next basho after that last completed one.
    pub fn current_or_next_basho_id(db: &Connection) -> SqlResult<BashoId> {
        let last_completed_basho: Option<BashoId> = db.query_row(
            "
            SELECT MAX(id)
            FROM basho AS b
            WHERE EXISTS (SELECT 1 FROM award AS a WHERE a.basho_id = b.id)
        ",
            [],
            |row| row.get(0),
        )?;
        Ok(last_completed_basho
            .map(|id| id.incr(1))
            .unwrap_or_else(|| VERY_FIRST_BASHO.parse().unwrap()))
    }

    pub fn with_id(db: &Connection, id: BashoId) -> Result<Option<BashoInfo>> {
        db.query_row("
            SELECT
                COUNT(*) AS n,
                basho.start_date,
                basho.venue,
                ebr.url AS external_link,
                CASE
                    WHEN ebr.basho_id IS NULL THEN
                        CASE
                            WHEN COUNT(br.player_id) = 0 THEN (
                                SELECT COUNT(DISTINCT player_id) FROM pick AS p WHERE p.basho_id = basho.id
                            )
                            ELSE COUNT(*)
                        END
                    ELSE ebr.players
                END AS player_count,
                COALESCE(MAX(br.wins), ebr.winning_score) AS winning_score
            FROM basho
            LEFT JOIN basho_result AS br ON br.basho_id = basho.id
            LEFT JOIN external_basho_result AS ebr ON ebr.basho_id = basho.id
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
                                 player_count: row.get::<_, u32>("player_count")? as usize,
                                 winning_score: row.get("winning_score")?,
                                 winners: BashoInfo::fetch_basho_winners(db, id)?,
                             }))
                         }
                     })
            .map_err(|e| e.into())
    }

    pub fn current_and_previous(db: &Connection) -> Result<(Option<BashoInfo>, Option<BashoInfo>)> {
        let mut stmt = db.prepare("
                SELECT
                    basho.id,
                    basho.start_date,
                    basho.venue,
                    ebr.url AS external_link,
                    CASE
                        WHEN ebr.basho_id IS NULL THEN
                            CASE
                                WHEN COUNT(br.player_id) = 0 THEN (
                                    SELECT COUNT(DISTINCT player_id) FROM pick AS p WHERE p.basho_id = basho.id
                                )
                                ELSE COUNT(*)
                            END
                        ELSE ebr.players
                    END AS player_count,
                    COALESCE(MAX(br.wins), ebr.winning_score) AS winning_score
                FROM basho
                LEFT JOIN basho_result AS br ON br.basho_id = basho.id
                LEFT JOIN external_basho_result AS ebr ON ebr.basho_id = basho.id
                GROUP BY basho.id
                ORDER BY basho.id DESC
                LIMIT 2")?;
        let mut infos = stmt.query_map([], |row| {
            let basho_id = row.get("id")?;
            Ok(BashoInfo {
                id: basho_id,
                start_date: row.get("start_date")?,
                venue: row.get("venue")?,
                external_link: row.get("external_link")?,
                player_count: row.get::<_, u32>("player_count")? as usize,
                winning_score: row.get("winning_score")?,
                winners: BashoInfo::fetch_basho_winners(db, basho_id)?,
            })
        })?;
        let first = infos.next_invert()?;
        let second = infos.next_invert()?;
        if let Some(f) = &first {
            if f.winners.is_empty() {
                Ok((first, second))
            } else {
                Ok((None, first))
            }
        } else {
            Ok((None, None))
        }
    }

    pub fn list_all(db: &Connection) -> Result<Vec<BashoInfo>> {
        let mut winners = BashoInfo::fetch_all_winners(db)?;
        db.prepare(
            "
                SELECT
                    basho.id,
                    basho.start_date,
                    basho.venue,
                    ebr.url AS external_link,
                    CASE
                        WHEN ebr.basho_id IS NULL THEN COUNT(DISTINCT br.player_id)
                        ELSE ebr.players
                    END AS player_count,
                    COALESCE(MAX(br.wins), ebr.winning_score) AS winning_score
                FROM basho
                LEFT JOIN basho_result AS br ON br.basho_id = basho.id
                LEFT JOIN external_basho_result AS ebr ON ebr.basho_id = basho.id
                GROUP BY basho.id
                ORDER BY basho.id DESC",
        )?
        .query_map([], |row| {
            let basho_id = row.get("id")?;
            Ok(BashoInfo {
                id: basho_id,
                start_date: row.get("start_date")?,
                venue: row.get("venue")?,
                external_link: row.get("external_link")?,
                player_count: row.get::<_, u32>("player_count")? as usize,
                winning_score: row.get("winning_score")?,
                winners: winners.remove(&basho_id).unwrap_or_default(),
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
        Ok(db
            .prepare(
                "
                SELECT p.*
                FROM award AS a
                JOIN player_info AS p ON p.id = a.player_id
                WHERE a.basho_id = ? AND a.type = ?
            ",
            )
            .unwrap()
            .query_map(params![basho_id, Award::EmperorsCup], Player::from_row)?
            .map(|r| r.unwrap())
            .collect())
    }

    fn fetch_all_winners(db: &Connection) -> SqlResult<HashMap<BashoId, Vec<Player>>> {
        let mut map = HashMap::new();
        let mut stmt = db.prepare(
            "
                SELECT
                    a.basho_id,
                    p.*
                FROM award AS a
                JOIN player_info AS p ON p.id = a.player_id
                WHERE a.type = ?
                ORDER BY basho_id DESC
            ",
        )?;
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

pub fn save_player_picks(
    db: &mut Connection,
    player_id: PlayerId,
    basho_id: BashoId,
    picks: [Option<RikishiId>; 5],
) -> Result<()> {
    let txn = db.transaction()?;
    let start_date: DateTime<Utc> = txn.query_row(
        "
        SELECT start_date
        FROM basho
        WHERE id = ?",
        params![basho_id],
        |row| row.get(0),
    )?;
    if start_date < Utc::now() {
        return Err(DataError::BashoHasStarted);
    }

    let rank_groups: Vec<RankGroup> = txn
        .prepare(
            "
        SELECT rank
        FROM banzuke
        WHERE basho_id = ? AND rikishi_id IN (?, ?, ?, ?, ?)",
        )?
        .query_map(
            params![basho_id, picks[0], picks[1], picks[2], picks[3], picks[4]],
            |row| row.get(0),
        )?
        .map(|rank: rusqlite::Result<Rank>| rank.unwrap().group())
        .collect();
    debug!("rank groups {:?} for picks {:?}", rank_groups, picks);
    if rank_groups
        .clone()
        .into_iter()
        .unique()
        .collect::<Vec<RankGroup>>()
        != rank_groups
    {
        return Err(DataError::InvalidPicks);
    }

    txn.execute(
        "
        DELETE FROM pick
        WHERE player_id = ? AND basho_id = ?",
        params![player_id, basho_id],
    )?;
    for rikishi_id in picks.iter().flatten() {
        debug!(
            "inserting player {} pick {} for {}",
            player_id, rikishi_id, basho_id
        );
        txn.execute(
            "
            INSERT INTO pick (player_id, basho_id, rikishi_id)
            VALUES (?, ?, ?)",
            params![player_id, basho_id, rikishi_id],
        )?;
    }
    txn.commit()?;

    Ok(())
}

pub fn update_basho(
    db: &mut Connection,
    basho_id: BashoId,
    venue: &str,
    start_date: &NaiveDateTime,
    banzuke: &[(String, Rank, bool)],
) -> Result<()> {
    let txn = db.transaction()?;
    txn.execute(
        "
        INSERT INTO basho (id, start_date, venue)
        VALUES (?, ?, ?)
        ON CONFLICT (id) DO UPDATE SET
            start_date = excluded.start_date,
            venue = excluded.venue
        ",
        params![basho_id, start_date, venue],
    )?;

    let mut rikishi_ids = HashMap::new();
    let mut given_names = HashMap::new();
    let query_str = format!(
        "
            SELECT id, family_name, given_name
            FROM rikishi
            WHERE family_name IN ({})
        ",
        banzuke.iter().map(|(_, _, _)| "?").join(", ")
    );
    let mut ambiguous_shikona = Vec::<String>::new();
    txn.prepare(query_str.as_str())?
        .query_map(
            params_from_iter(banzuke.iter().map(|(name, _, _)| name)),
            |row| {
                let id: i64 = row.get("id")?;
                let family_name: String = row.get("family_name")?;
                let given_name: String = row.get("given_name")?;
                if rikishi_ids.contains_key(&family_name) {
                    ambiguous_shikona.push(family_name.to_owned());
                }
                rikishi_ids.insert(family_name, id);
                given_names.insert(id, given_name);
                Ok(())
            },
        )?
        // force evaluation of mapping function and collapse errors into one Result
        .collect::<SqlResult<()>>()
        .map_err(DataError::from)?;
    if !ambiguous_shikona.is_empty() {
        return Err(DataError::AmbiguousShikona {
            family_names: ambiguous_shikona,
        });
    }

    for (family_name, rank, is_kyujyo) in banzuke {
        let rikishi_id = match rikishi_ids.get(family_name) {
            Some(id) => id.to_owned(),
            None => {
                txn.execute(
                    "
                        INSERT INTO rikishi (family_name, given_name)
                        VALUES (?, ?)
                    ",
                    params![family_name, ""],
                )?; // TODO given_name
                txn.last_insert_rowid()
            }
        };
        let given_name = given_names
            .get(&rikishi_id)
            .unwrap_or(&"".to_string())
            .to_owned(); // TODO given_name
        txn.execute(
            "
                INSERT INTO banzuke (rikishi_id, basho_id, family_name, given_name, rank, kyujyo)
                VALUES (?, ?, ?, ?, ?, ?)
                ON CONFLICT (rikishi_id, basho_id) DO UPDATE SET
                    family_name = excluded.family_name,
                    given_name = excluded.given_name,
                    rank = excluded.rank,
                    kyujyo = excluded.kyujyo
            ",
            params![
                rikishi_id,
                basho_id,
                family_name,
                given_name,
                rank,
                is_kyujyo
            ],
        )?;
    }
    txn.commit()?;

    Ok(())
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
pub struct TorikumiMatchUpdateData {
    pub winner: String,
    pub loser: String,
}

pub fn update_torikumi(
    db: &mut Connection,
    basho_id: BashoId,
    day: Day,
    torikumi: &[TorikumiMatchUpdateData],
) -> Result<()> {
    debug!("updating torikumi for {} day {}", basho_id, day);

    let txn = db.transaction()?;

    let mut rikishi_ids = HashMap::new();
    let mut rikishi_ranks = HashMap::new();
    let mut ambiguous_shikona = Vec::<String>::new();
    txn.prepare(
        "
            SELECT b.rikishi_id, b.family_name, b.rank
            FROM banzuke AS b
            WHERE b.basho_id = ?
        ",
    )?
    .query_map(params![basho_id], |row| {
        let id: i64 = row.get("rikishi_id")?;
        let family_name: String = row.get("family_name")?;
        let rank: Rank = row.get("rank")?;
        trace!("found mapping {} to rikishi id {}", family_name, id);
        if rikishi_ids.contains_key(&family_name) {
            ambiguous_shikona.push(family_name.to_owned());
        }
        rikishi_ids.insert(family_name, id);
        rikishi_ranks.insert(id, rank);
        Ok(())
    })?
    // force evaluation of mapping function and collapse errors into one Result
    .collect::<SqlResult<()>>()
    .map_err(DataError::from)?;
    if !ambiguous_shikona.is_empty() {
        return Err(DataError::AmbiguousShikona {
            family_names: ambiguous_shikona,
        });
    }

    txn.execute(
        "
            DELETE FROM torikumi
            WHERE basho_id = ? AND day = ?
        ",
        params![basho_id, day],
    )?;

    for (seq, TorikumiMatchUpdateData { winner, loser }) in torikumi.iter().enumerate() {
        let winner_id = rikishi_ids
            .get(winner)
            .ok_or_else(|| DataError::RikishiNotFound {
                family_name: winner.to_owned(),
            })?;
        let loser_id = rikishi_ids
            .get(loser)
            .ok_or_else(|| DataError::RikishiNotFound {
                family_name: loser.to_owned(),
            })?;
        let winner_rank = rikishi_ranks.get(winner_id).unwrap();
        let loser_rank = rikishi_ranks.get(loser_id).unwrap();

        let insert_1 = |side, rikishi_id, win| {
            txn.execute(
                "
                    INSERT INTO torikumi (basho_id, day, seq, side, rikishi_id, win)
                    VALUES (?, ?, ?, ?, ?, ?)
                ",
                params![basho_id, day, seq as u32, side, rikishi_id, win],
            )
        };

        // Figuring out the side: the rikishi with the higher rank appear on their own rank.side
        insert_1(
            if winner_rank > loser_rank {
                winner_rank.side
            } else {
                loser_rank.side.other()
            },
            winner_id,
            true,
        )?;
        insert_1(
            if loser_rank > winner_rank {
                loser_rank.side
            } else {
                winner_rank.side.other()
            },
            loser_id,
            false,
        )?;
    }

    upsert_basho_results(&txn, basho_id, false)?;

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
    pub is_kyujyo: bool,
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

    pub fn result_chunks(&self) -> Vec<&[Option<bool>]> {
        self.results.chunks(5).collect()
    }
}

pub struct BashoRikishiByRank {
    pub rank: String,
    pub rank_group: RankGroup,
    pub east: Option<BashoRikishi>,
    pub west: Option<BashoRikishi>,
}

impl BashoRikishiByRank {
    pub fn make_boundary() -> Self {
        Self {
            rank: "boundary".to_string(),
            rank_group: RankGroup(u8::MAX),
            east: None,
            west: None,
        }
    }

    pub fn next_day(&self) -> u8 {
        max(
            self.east.as_ref().map_or(1, |r| r.next_day()),
            self.west.as_ref().map_or(1, |r| r.next_day()),
        )
    }
}

pub struct FetchBashoRikishi {
    pub by_rank: Vec<BashoRikishiByRank>,
    pub by_id: HashMap<RikishiId, BashoRikishi>,
}

impl FetchBashoRikishi {
    pub fn with_db(db: &Connection, basho_id: BashoId, picks: &HashSet<RikishiId>) -> Result<Self> {
        debug!("fetching rikishi results for basho {}", basho_id);
        struct FetchedRikishiRow(
            Rank,
            RikishiId,
            String,
            bool,
            Option<Day>,
            Option<bool>,
            u16,
        );
        let mut vec: Vec<BashoRikishiByRank> = db
            .prepare(
                "
            SELECT
                banzuke.rank,
                banzuke.rikishi_id,
                banzuke.family_name,
                banzuke.kyujyo,
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
        ",
            )
            .unwrap()
            .query_map(params![basho_id], |row| -> SqlResult<FetchedRikishiRow> {
                Ok(FetchedRikishiRow(
                    row.get("rank")?,
                    row.get("rikishi_id")?,
                    row.get("family_name")?,
                    row.get("kyujyo")?,
                    row.get("day")?,
                    row.get("win")?,
                    row.get("picks")?,
                ))
            })?
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
                        picks: arow.6,
                        is_player_pick: picks.contains(&arow.1),
                        is_kyujyo: arow.3,
                    };
                    for FetchedRikishiRow(_, _, _, _, day, win, _) in rows {
                        match win {
                            Some(true) => rikishi.wins += 1,
                            Some(false) => rikishi.losses += 1,
                            None => (),
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
        let mut boundaries: Vec<_> = vec
            .iter()
            .tuple_windows()
            .map(|(x, y)| x.rank_group != y.rank_group)
            .collect();
        boundaries.reverse();
        let orig_vec_len = vec.len();
        for (i, is_boundary) in boundaries.into_iter().enumerate() {
            if is_boundary {
                let insert_idx = orig_vec_len - i - 1;
                vec.insert(insert_idx, BashoRikishiByRank::make_boundary());
            }
        }

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

pub fn finalize_basho(db: &mut Connection, basho_id: BashoId) -> Result<()> {
    debug!("finalizing basho {}", basho_id);
    let txn = db.transaction()?;
    upsert_basho_results(&txn, basho_id, true)?;
    upsert_player_ranks(&txn, basho_id)?;
    debug!("committing");
    txn.commit()?;
    Ok(())
}

pub fn backfill_past_player_ranks(db: &mut Connection, to_basho: BashoId) -> Result<()> {
    let first_basho = VERY_FIRST_BASHO.parse().unwrap();
    info!(
        "backfill_past_player_ranks from {} to {}",
        first_basho, to_basho
    );

    let txn = db.transaction()?;
    let mut basho_id = first_basho;
    while basho_id <= to_basho {
        upsert_player_ranks(&txn, basho_id)?;
        basho_id = basho_id.incr(1);
    }
    txn.commit()?;
    Ok(())
}

fn upsert_basho_results(txn: &Transaction, basho_id: BashoId, bestow_awards: bool) -> Result<()> {
    info!(
        "upsert_basho_results for {}; bestow_awards: {}",
        basho_id, bestow_awards
    );
    let scores = BashoPlayerScore::fetch(txn, basho_id)?;

    if bestow_awards {
        let count = txn
            .prepare(
                "
                DELETE FROM award
                WHERE basho_id = ? AND type = ?
            ",
            )?
            .execute(params![basho_id, Award::EmperorsCup])?;
        debug!("deleted {} previously bestowed emperor's cups", count);
    }

    // For each player, upsert basho_result and award emperor's cup if they ranked #1
    let mut insert_result_stmt = txn.prepare(
        "
            INSERT INTO basho_result (basho_id, player_id, wins, rank) VALUES (?, ?, ?, ?)
            ON CONFLICT (basho_id, player_id) DO UPDATE
            SET wins = excluded.wins,
                rank = excluded.rank
        ",
    )?;
    let mut insert_award_stmt = txn.prepare(
        "
            INSERT INTO award (basho_id, player_id, type)
            VALUES (?, ?, ?)
        ",
    )?;
    for p in scores {
        trace!(
            "- rank {} player {} ({}) with {} wins",
            p.rank,
            p.name,
            p.id,
            p.wins
        );
        insert_result_stmt.execute(params![basho_id, p.id, p.wins, p.rank as u32])?;
        if bestow_awards && p.rank == 1 {
            debug!("  ! awarding emperor's cup to {}", p.name);
            insert_award_stmt.execute(params![basho_id, p.id, Award::EmperorsCup])?;
        }
    }
    Ok(())
}

fn upsert_player_ranks(txn: &Transaction, last_basho: BashoId) -> Result<()> {
    let basho_range = last_basho.next().range_for_banzuke();
    let leaders = HistoricLeader::with_basho_range(txn, &basho_range, u32::MAX)?;
    info!(
        "upsert_player_ranks for {} players after basho {}",
        leaders.len(),
        last_basho
    );
    let mut insert_rank_stmt = txn.prepare(
        "
            INSERT INTO player_rank (player_id, before_basho_id, rank, past_year_wins)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (player_id, before_basho_id) DO UPDATE
            SET rank = excluded.rank,
                past_year_wins = excluded.past_year_wins
        ",
    )?;
    for l in leaders {
        trace!(
            "- player {} ({}) ranked {} with {} wins",
            l.player.id,
            l.player.name,
            l.rank,
            l.wins.total.unwrap_or(0)
        );
        insert_rank_stmt.execute(params![
            l.player.id,
            basho_range.end,
            l.rank,
            l.wins.total.unwrap_or(0)
        ])?;
    }
    Ok(())
}

struct BashoPlayerScore {
    id: PlayerId,
    name: String,
    wins: u8,
    rank: usize,
}

impl Rankable for BashoPlayerScore {
    fn get_score(&self) -> i32 {
        self.wins as i32
    }

    fn set_rank(&mut self, ord: usize) {
        self.rank = ord
    }
}

impl BashoPlayerScore {
    fn fetch(txn: &Transaction, basho_id: BashoId) -> Result<Vec<Self>> {
        let mut players: Vec<Self> = txn
            .prepare(
                "
                SELECT
                    p.id,
                    p.name,
                    bs.wins
                FROM basho_score AS bs
                JOIN player AS p ON p.id = bs.player_id
                WHERE bs.basho_id = ?
                ORDER BY bs.wins DESC
            ",
            )?
            .query_map(params![basho_id], |row| -> SqlResult<Self> {
                Ok(Self {
                    id: row.get("id")?,
                    name: row.get("name")?,
                    wins: row.get("wins")?,
                    rank: 0,
                })
            })?
            .collect::<SqlResult<_>>()?;
        assign_ord(&mut players.iter_mut());
        Ok(players)
    }
}
