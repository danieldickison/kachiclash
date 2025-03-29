use rusqlite::{Connection, Result as SqlResult};
use std::{collections::HashMap, ops::Range};

use super::{
    heya::HeyaId, BashoId, BashoRikishi, Player, PlayerId, Rank, RankName, RankSide, Result,
    RikishiId,
};
use crate::util::GroupRuns;
use std::sync::Arc;

pub struct BashoPlayerResults {
    pub player: ResultPlayer,
    pub total: u8,
    pub days: [Option<u8>; 15],
    picks: [Option<RikishiId>; 5],
    rikishi_by_id: Arc<HashMap<RikishiId, BashoRikishi>>,
    pub is_self: bool,
}

#[allow(clippy::large_enum_variant)]
pub enum ResultPlayer {
    RankedPlayer(Player, usize),
    Max,
    Min,
}

impl BashoPlayerResults {
    // how to make this work with askama borrow semantics with -> impl Iterator<Item = Option<&BashoRikishi>>
    pub fn picks(&self) -> Vec<Option<&BashoRikishi>> {
        self.picks
            .iter()
            .map(move |opt_id| opt_id.and_then(move |id| self.rikishi_by_id.get(&id)))
            .collect()
    }

    fn sort_key_during_basho(&self) -> impl Ord {
        match &self.player {
            ResultPlayer::RankedPlayer(_, basho_rank) => (
                if *basho_rank == 0 { 1 } else { *basho_rank },
                self.picks()
                    .iter()
                    .map(|rikishi| rikishi.map_or("".to_string(), |r| r.name.clone()))
                    .collect(),
            ),
            ResultPlayer::Max => (0, vec![]),
            ResultPlayer::Min => (usize::MAX, vec![]),
        }
    }
    fn sort_key_before_basho(&self) -> impl Ord {
        match &self.player {
            ResultPlayer::RankedPlayer(player, _) => (
                player.rank.unwrap_or(Rank::bottom()),
                player.name.to_lowercase(),
            ),
            _ => (Rank::bottom(), "".to_string()),
        }
    }

    pub fn fetch(
        db: &Connection,
        basho_id: BashoId,
        player_id: Option<PlayerId>,
        rikishi: HashMap<RikishiId, BashoRikishi>,
        include_best_worst: bool,
        limit: usize,
        heya_id: Option<HeyaId>,
    ) -> Result<Vec<Self>> {
        debug!(
            "fetching {} leaders for basho {} heya {:?}",
            limit, basho_id, heya_id
        );

        let rikishi = Arc::new(rikishi);
        let (heya_join, params) = if heya_id.is_some() {
            (
                "JOIN heya_player AS hp ON hp.player_id = player.id AND hp.heya_id = :heya_id",
                named_params! {
                    ":basho_id": basho_id,
                    ":player_id": player_id,
                    ":limit": limit as u32,
                    ":heya_id": heya_id,
                },
            )
        } else {
            (
                "",
                named_params! {
                    ":basho_id": basho_id,
                    ":player_id": player_id,
                    ":limit": limit as u32
                },
            )
        };
        let mut leaders: Vec<BashoPlayerResults> = db
            .prepare(
                &format!(
                "
                    SELECT
                        player.*,
                        pr.rank,
                        COALESCE(br.wins, 0) AS basho_wins,
                        COALESCE(br.rank, 0) AS basho_rank,
                        player.id = :player_id AS is_self,
                        GROUP_CONCAT(pick.rikishi_id) AS pick_ids
                    FROM pick
                    JOIN player_info AS player ON player.id = pick.player_id
                    {heya_join}
                    LEFT JOIN player_rank AS pr ON pr.player_id = player.id AND pr.before_basho_id = pick.basho_id
                    LEFT JOIN basho_result AS br USING (player_id, basho_id)
                    WHERE pick.basho_id = :basho_id
                    GROUP BY player.id
                    ORDER BY is_self DESC, basho_wins DESC, player.id ASC
                    LIMIT :limit
                ")
            )
            .unwrap()
            .query_map(
               params,
                |row| -> SqlResult<(Player, u8, u32, String)> {
                    Ok((
                        Player::from_row(row)?,
                        row.get("basho_wins")?,
                        row.get("basho_rank")?,
                        row.get("pick_ids")?,
                    ))
                },
            )?
            .collect::<SqlResult<Vec<(Player, u8, u32, String)>>>()?
            .into_iter()
            .map(|(player, total, rank, picks_str)| {
                let mut picks = [None; 5];
                let mut pick_rikishi = [None; 5];
                for r in picks_str
                    .split(',')
                    .filter_map(|id| rikishi.get(&id.parse().unwrap()))
                {
                    let group = r.rank.group().as_index();
                    picks[group] = Some(r.id);
                    pick_rikishi[group] = Some(r);
                }
                let (days, total_validation) = picks_to_days(&pick_rikishi);
                if total != total_validation {
                    warn!("total wins for player {} mismatch betwen basho_result {total} and live data {total_validation}", player.name)
                }
                BashoPlayerResults {
                    is_self: player_id.map_or(false, |id| player.id == id),
                    player: ResultPlayer::RankedPlayer(player, rank as usize),
                    rikishi_by_id: Arc::clone(&rikishi),
                    picks,
                    total,
                    days,
                }
            })
            .collect();

        if include_best_worst {
            let (min, max) = make_min_max_results(rikishi);
            leaders.push(min);
            leaders.push(max);
            // Sort to put self player in correct place. (It's always first from the db query to ensure it doesn't get bumped off by the LIMIT clause.)
            leaders.sort_by_cached_key(|p| p.sort_key_during_basho());
        } else {
            leaders.sort_by_cached_key(|p| p.sort_key_before_basho())
        }

        Ok(leaders)
    }
}

fn make_min_max_results(
    rikishi: Arc<HashMap<RikishiId, BashoRikishi>>,
) -> (BashoPlayerResults, BashoPlayerResults) {
    let mut mins = [None; 5];
    let mut maxes = [None; 5];
    for r in rikishi.values().filter(|r| !r.is_kyujyo) {
        let group = r.rank.group().as_index();
        mins[group] = mins[group].map_or(Some(r), |min: &BashoRikishi| {
            Some(if r.wins < min.wins { r } else { min })
        });
        maxes[group] = maxes[group].map_or(Some(r), |max: &BashoRikishi| {
            Some(if r.wins > max.wins { r } else { max })
        });
    }
    let (min_days, min_total) = picks_to_days(&mins);
    let (max_days, max_total) = picks_to_days(&maxes);
    let mut min_ids = [None; 5];
    let mut max_ids = [None; 5];
    for i in 0..5 {
        min_ids[i] = mins[i].map(|r| r.id);
        max_ids[i] = maxes[i].map(|r| r.id);
    }

    (
        BashoPlayerResults {
            is_self: false,
            player: ResultPlayer::Min,
            picks: min_ids,
            rikishi_by_id: Arc::clone(&rikishi),
            total: min_total,
            days: min_days,
        },
        BashoPlayerResults {
            is_self: false,
            player: ResultPlayer::Max,
            picks: max_ids,
            rikishi_by_id: Arc::clone(&rikishi),
            total: max_total,
            days: max_days,
        },
    )
}

fn picks_to_days(picks: &[Option<&BashoRikishi>; 5]) -> ([Option<u8>; 15], u8) {
    let mut days = [None; 15];
    let mut total_validation = 0;
    for pick in picks.iter().flatten() {
        for (day, win) in pick.results.iter().enumerate() {
            if let Some(win) = win {
                let incr = if *win { 1 } else { 0 };
                days[day] = Some(days[day].unwrap_or(0) + incr);
                total_validation += incr;
            }
        }
    }
    (days, total_validation)
}

pub struct PlayerRanking {
    pub player: Player,
    pub ord: usize,
    pub rank: Rank,
    pub wins: u32,
}

impl Rankable for PlayerRanking {
    fn get_score(&self) -> i32 {
        self.wins as i32
    }

    fn set_rank(&mut self, ord: usize) {
        self.ord = ord;
    }
}

impl PlayerRanking {
    pub fn for_home_page(db: &Connection, next_basho_id: BashoId) -> Result<Vec<Self>> {
        let mut rows = db
            .prepare(
                "
            SELECT
                p.*,
                pr.rank,
                pr.past_year_wins
            FROM player_rank AS pr
            JOIN player_info AS p ON p.id = pr.player_id
            WHERE pr.before_basho_id = ?
            ORDER BY past_year_wins DESC, LOWER(name) ASC
        ",
            )?
            .query_map(params![next_basho_id], |row| {
                Ok(Self {
                    player: Player::from_row(row)?,
                    ord: 0, // set later
                    rank: row.get("rank")?,
                    wins: row.get("past_year_wins")?,
                })
            })?
            .collect::<rusqlite::Result<Vec<Self>>>()?;
        assign_ord(&mut rows.iter_mut());
        Ok(rows)
    }
}

#[derive(Debug)]
pub struct HistoricLeader {
    pub player: Player,
    pub ord: usize,
    pub rank: Rank,
    pub wins: NumericStats,
    pub ranks: NumericStats,
}

#[derive(Debug, Clone, Copy)]
pub struct NumericStats {
    pub total: Option<u32>,
    pub min: Option<u32>,
    pub max: Option<u32>,
    pub mean: Option<f64>,
}

impl Rankable for HistoricLeader {
    fn get_score(&self) -> i32 {
        self.wins.total.unwrap_or(0) as i32
    }

    fn set_rank(&mut self, ord: usize) {
        self.ord = ord;
    }
}

impl HistoricLeader {
    pub fn with_basho_range(
        db: &Connection,
        range: &Range<BashoId>,
        player_limit: u32,
    ) -> Result<Vec<Self>> {
        debug!("Fetching {} leaders in {:?}", player_limit, range);
        let mut leaders = db.prepare("
                SELECT
                    p.*,
                    SUM(r.wins) AS total_wins,
                    MIN(r.wins) AS min_wins,
                    MAX(r.wins) AS max_wins,
                    AVG(r.wins) AS mean_wins,
                    MIN(r.rank) AS min_rank,
                    MAX(r.rank) AS max_rank,
                    AVG(r.rank) AS mean_rank
                FROM (
                    SELECT p.id, r.basho_id, r.wins, r.rank
                    FROM player AS p
                    JOIN basho_result AS r ON r.player_id = p.id AND r.basho_id >= ? AND r.basho_id < ?

                    UNION ALL

                    SELECT p.id, e.basho_id, e.wins, e.rank
                    FROM player AS p
                    JOIN external_basho_player AS e ON e.name = p.name AND e.basho_id >= ? AND e.basho_id < ?
                ) AS r
                JOIN player_info AS p ON p.id = r.id
                GROUP BY p.id
                ORDER BY total_wins DESC, max_wins DESC, min_rank ASC NULLS LAST
                LIMIT ?
            ")?
            .query_and_then(
                params![range.start, range.end, range.start, range.end, player_limit],
                |row| Ok(Self {
                    player: Player::from_row(row)?,
                    ord: 0,
                    rank: Rank::top(),
                    wins: NumericStats {
                        total: row.get("total_wins")?,
                        min: row.get("min_wins")?,
                        max: row.get("max_wins")?,
                        mean: row.get("mean_wins")?,
                    },
                    ranks: NumericStats {
                        total: None,
                        min: row.get("min_rank")?,
                        max: row.get("max_rank")?,
                        mean: row.get("mean_rank")?,
                    }
                })
            )?
            .collect::<Result<Vec<Self>>>()?;
        assign_ord(&mut leaders.iter_mut());
        Self::assign_rank(&mut leaders);
        Ok(leaders)
    }

    fn assign_rank(leaders: &mut [HistoricLeader]) {
        let mut rank = Rank {
            name: RankName::Yokozuna,
            side: RankSide::East,
            number: 1,
        };
        for group in leaders.group_runs_mut(|a, b| a.ord == b.ord) {
            for leader in group.iter_mut() {
                leader.rank = rank;
            }
            rank = rank.next_lower();
        }
    }
}

pub trait Rankable {
    fn get_score(&self) -> i32;
    fn set_rank(&mut self, ord: usize);
}

pub fn assign_ord<'a, I, R>(iter: &'a mut I)
where
    I: Iterator<Item = &'a mut R>,
    R: Rankable + 'a,
{
    let mut last_score = i32::MIN;
    let mut ord = 1;

    for (i, r) in iter.enumerate() {
        if r.get_score() != last_score {
            last_score = r.get_score();
            ord = i + 1;
        }
        r.set_rank(ord);
    }
}
