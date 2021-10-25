use std::collections::HashMap;
use rusqlite::{Connection, Result as SqlResult};

use super::{BashoId, BashoRikishi, Player, PlayerId, Rank, RankName, RankSide, Result, RikishiId};
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

    fn sort_key(&self) -> (usize, Vec<String>) {
        match &self.player {
            ResultPlayer::RankedPlayer(_, rank) => {
                (
                    if *rank == 0 { 1 } else { *rank },
                    self.picks().iter().map(|rikishi|
                        rikishi.map_or("".to_string(), |r| r.name.clone())
                    ).collect()
                )
            },
            ResultPlayer::Max => (0, vec![]),
            ResultPlayer::Min => (usize::max_value(), vec![]),
        }
    }

    pub fn fetch(db: &Connection,
                 basho_id: BashoId,
                 player_id: Option<PlayerId>,
                 rikishi: HashMap<RikishiId, BashoRikishi>,
                 include_best_worst: bool,
                 limit: usize)
        -> Result<Vec<Self>> {

        debug!("fetching {} leaders for basho {}", limit, basho_id);

        let rikishi = Arc::new(rikishi);
        let mut leaders: Vec<BashoPlayerResults> = db.prepare("
                SELECT
                    player.*,
                    COALESCE(br.wins, 0) AS wins,
                    COALESCE(br.rank, 0) AS rank,
                    player.id = :player_id AS is_self,
                    GROUP_CONCAT(pick.rikishi_id) AS pick_ids
                FROM pick
                JOIN player_info AS player ON player.id = pick.player_id
                LEFT NATURAL JOIN basho_result AS br
                WHERE pick.basho_id = :basho_id
                GROUP BY player.id
                ORDER BY is_self DESC, wins DESC, player.id ASC
                LIMIT :limit
            ").unwrap()
            .query_map(
                named_params! {
                    ":basho_id": basho_id,
                    ":player_id": player_id,
                    ":limit": limit as u32,
                },
                |row| -> SqlResult<(Player, u8, u32, String)> {
                    Ok((
                        Player::from_row(row)?,
                        row.get("wins")?,
                        row.get("rank")?,
                        row.get("pick_ids")?,
                    ))
                }
            )?
            .collect::<SqlResult<Vec<(Player, u8, u32, String)>>>()?
            .into_iter()
            .map(|(player, total, rank, picks_str)| {
                let mut picks = [None; 5];
                let mut pick_rikishi = [None; 5];
                for r in picks_str.split(',').filter_map(|id| rikishi.get(&id.parse().unwrap())) {
                    let group = r.rank.group().as_index();
                    picks[group] = Some(r.id);
                    pick_rikishi[group] = Some(r);
                }
                let (days, total_validation) = picks_to_days(&pick_rikishi);
                assert_eq!(total, total_validation);
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
        }

        // Sort to put self player at the bottom. (It's always first from the db query to ensure it doesn't get bumped off by the LIMIT clause.)
        leaders.sort_by_cached_key(|p| p.sort_key());

        Ok(leaders)
    }
}

fn make_min_max_results(rikishi: Arc<HashMap<RikishiId, BashoRikishi>>)
                        -> (BashoPlayerResults, BashoPlayerResults) {
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
    pub fn with_first_basho(db: &Connection, first_basho: Option<BashoId>, player_limit: u32) -> Result<Vec<Self>> {
        let first_basho = first_basho.unwrap_or_else(|| "201901".parse().unwrap());
        debug!("Fetching {} leaders since basho {}", player_limit, first_basho);
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
                    LEFT JOIN basho_result AS r ON r.player_id = p.id AND r.basho_id >= ?

                    UNION ALL

                    SELECT p.id, e.basho_id, e.wins, e.rank
                    FROM player AS p
                    LEFT JOIN external_basho_player AS e ON e.name = p.name AND e.basho_id >= ?
                ) AS r
                JOIN player_info AS p ON p.id = r.id
                GROUP BY p.id
                ORDER BY total_wins DESC, max_wins DESC, min_rank ASC NULLS LAST
                LIMIT ?
            ")?
            .query_and_then(
                params![first_basho, first_basho, player_limit],
                |row| Ok(Self {
                    player: Player::from_row(&row)?,
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
        let mut rank = Rank {name: RankName::Yokozuna, side: RankSide::East, number: 1};
        let mut count = 0;
        let mut iter = leaders.group_runs_mut(|a, b| a.ord == b.ord).peekable();
        while let Some(group) = iter.next() {
            for leader in group.iter_mut() {
                leader.rank = rank;
            }
            count = count + group.len();

            let next_len = iter.peek().map_or(0, |next| next.len());
            let RankPlayerCounts {minimum, preferred} = RankPlayerCounts::for_rank(rank);
            if count >= preferred ||
                (count >= minimum && count + next_len > preferred)
            {
                rank = rank.next_lower();
                count = 0;
            }
        }
    }
}

pub trait Rankable {
    fn get_score(&self) -> i32;
    fn set_rank(&mut self, ord: usize);
}

pub fn assign_ord<'a, I, R: 'a>(iter: &'a mut I)
where
    I: Iterator<Item=&'a mut R>,
    R: Rankable
{
    let mut last_score = i32::min_value();
    let mut ord = 1;

    for (i, r) in iter.enumerate() {
        if r.get_score() != last_score {
            last_score = r.get_score();
            ord = i + 1;
        }
        r.set_rank(ord);
    }
}

struct RankPlayerCounts {
    minimum: usize,
    preferred: usize,
}

impl RankPlayerCounts {
    fn for_rank(rank: Rank) -> Self {
        match rank.name {
            RankName::Yokozuna  => Self {minimum: 1, preferred: 1},
            RankName::Ozeki     => Self {minimum: 1, preferred: 2},
            RankName::Sekiwake  => Self {minimum: 1, preferred: 4},
            RankName::Komusubi  => Self {minimum: 1, preferred: 4},
            RankName::Maegashira=> Self {minimum: 3, preferred: 4},
            RankName::Juryo     => Self {minimum: 4, preferred: 4},
        }
    }
}
