use std::collections::HashMap;
use rusqlite::{Connection, Result as SqlResult};

use super::{Result, PlayerId, Player, RikishiId, BashoId, BashoRikishi};
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

    fn sort_rank(&self) -> usize {
        match self.player {
            ResultPlayer::RankedPlayer(_, rank) => rank,
            ResultPlayer::Max => 0,
            ResultPlayer::Min => usize::max_value(),
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
                    br.wins,
                    br.rank,
                    player.id = :player_id AS is_self,
                    GROUP_CONCAT(pick.rikishi_id) AS pick_ids
                FROM pick
                JOIN player_info AS player ON player.id = pick.player_id
                LEFT NATURAL JOIN basho_result AS br
                WHERE pick.basho_id = :basho_id
                GROUP BY player.id
                ORDER BY is_self DESC, wins DESC
                LIMIT :limit
            ").unwrap()
            .query_map_named(
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
        leaders.sort_by_key(|p| p.sort_rank());

        Ok(leaders)
    }
}

fn make_min_max_results(rikishi: Arc<HashMap<RikishiId, BashoRikishi>>)
                        -> (BashoPlayerResults, BashoPlayerResults) {
    let mut mins = [None; 5];
    let mut maxes = [None; 5];
    for r in rikishi.values() {
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
    for pick in picks {
        if let Some(r) = pick {
            for (day, win) in r.results.iter().enumerate() {
                if let Some(win) = win {
                    let incr = if *win { 1 } else { 0 };
                    days[day] = Some(days[day].unwrap_or(0) + incr);
                    total_validation += incr;
                }
            }
        }
    }
    (days, total_validation)
}
