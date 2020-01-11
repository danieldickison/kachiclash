use std::collections::HashMap;
use rusqlite::{Connection, Result as SqlResult};

use super::{Result, PlayerId, Player, RikishiId, BashoId, BashoRikishi};

pub struct BashoPlayerResults<'a> {
    pub player: ResultPlayer,
    pub total: u8,
    pub days: [Option<u8>; 15],
    picks: [Option<RikishiId>; 5],
    rikishi_by_id: &'a HashMap<RikishiId, BashoRikishi>,
    pub rank: usize,
    pub is_self: bool,
}

pub enum ResultPlayer {
    RealPlayer(Player),
    Max,
    Min,
}

impl <'a> BashoPlayerResults<'a> {
    pub fn picks(&self) -> impl Iterator<Item = Option<&BashoRikishi>> {
        self.picks.iter().map(move |opt_id| opt_id.and_then(move |id| self.rikishi_by_id.get(&id)))
    }

    pub fn fetch(db: &Connection, basho_id: BashoId, player_id: Option<PlayerId>, rikishi: &'a HashMap<RikishiId, BashoRikishi>, include_best_worst: bool)
                     -> Result<Vec<Self>> {
        const LIMIT: usize = 300;
        debug!("fetching {} leaders for basho {}", LIMIT, basho_id);
        let mut leaders: Vec<BashoPlayerResults<'a>> = db.prepare("
                SELECT
                    player.*,
                    bs.wins,
                    player.id = :player_id AS is_self,
                    GROUP_CONCAT(pick.rikishi_id) AS pick_ids
                FROM basho_score AS bs
                JOIN player_info AS player ON player.id = bs.player_id
                LEFT JOIN player_discord AS discord ON discord.player_id = player.id
                JOIN pick ON pick.player_id = player.id AND pick.basho_id = bs.basho_id
                WHERE bs.basho_id = :basho_id
                GROUP BY player.id
                ORDER BY is_self DESC, bs.wins DESC
                LIMIT :limit
            ").unwrap()
            .query_map_named(
                named_params! {
                    ":basho_id": basho_id,
                    ":player_id": player_id,
                    ":limit": LIMIT as u32,
                },
                |row| -> SqlResult<(Player, u8, String)> {
                    Ok((
                        Player::from_row(row)?,
                        row.get("wins")?,
                        row.get("pick_ids")?,
                    ))
                }
            )?
            .collect::<SqlResult<Vec<(Player, u8, String)>>>()?
            .into_iter()
            .map(|(player, total, picks_str)| {
                let mut picks = [None; 5];
                let mut pick_rikishi = [None; 5];
                for r in picks_str.split(',').filter_map(|id| rikishi.get(&id.parse().unwrap())) {
                    let group = *r.rank.group() as usize - 1;
                    picks[group] = Some(r.id);
                    pick_rikishi[group] = Some(r);
                }
                let (days, total_validation) = picks_to_days(&pick_rikishi);
                assert_eq!(total, total_validation);
                BashoPlayerResults {
                    is_self: player_id.map_or(false, |id| player.id == id),
                    rank: 0, // populated later
                    player: ResultPlayer::RealPlayer(player),
                    rikishi_by_id: rikishi,
                    picks,
                    total,
                    days,
                }
            })
            .collect();

        // Sort and assign ranks
        leaders.sort_by_key(|p| -i16::from(p.total));
        let mut last_total = 0;
        let mut last_rank = 1;
        let count = leaders.len();
        for (i, p) in leaders.iter_mut().enumerate() {
            if p.total != last_total {
                last_total = p.total;
                last_rank = i + 1;
            }
            if p.is_self && count == LIMIT && i == count - 1 {
                p.rank = 0; // zero means an unknown rank > LIMIT
            } else {
                p.rank = last_rank;
            }
        }

        if include_best_worst {
            let (min, max) = make_min_max_results(&rikishi);
            leaders.insert(0, max);
            leaders.push(min);
        }

        Ok(leaders)
    }
}

fn make_min_max_results(rikishi: &HashMap<RikishiId, BashoRikishi>)
                        -> (BashoPlayerResults, BashoPlayerResults) {
    let mut mins = [None; 5];
    let mut maxes = [None; 5];
    for r in rikishi.values() {
        let group = *r.rank.group() as usize - 1;
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
            rank: 0, // n/a
            player: ResultPlayer::Min,
            picks: min_ids,
            rikishi_by_id: rikishi,
            total: min_total,
            days: min_days,
        },
        BashoPlayerResults {
            is_self: false,
            rank: 0, // n/a
            player: ResultPlayer::Max,
            picks: max_ids,
            rikishi_by_id: rikishi,
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
