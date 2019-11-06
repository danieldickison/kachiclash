extern crate itertools;
use std::collections::{HashSet, HashMap};
use actix_identity::Identity;
use itertools::Itertools;
use rusqlite::{Connection, Result as SqlResult};

use super::{BaseTemplate, Result, HandlerError, AskamaResponder};
use crate::data::{self, Rank, RankSide, RankGroup, BashoId, BashoInfo, PlayerId, RikishiId, Day, Player};
use crate::AppState;

use actix_web::{web, HttpResponse, Responder};
use askama::Template;
use failure::_core::cmp::max;


mod filters {
    use chrono::{DateTime, Utc, FixedOffset};

    static JST_OFFSET: i32 = 9 * 3600;

    pub fn jst_month_day(s: &DateTime<Utc>) -> askama::Result<String> {
        Ok(s.with_timezone(&FixedOffset::east(JST_OFFSET)).format("%B %-d").to_string())
    }
}

#[derive(Template)]
#[template(path = "basho_list.html")]
pub struct BashoListTemplate {
    base: BaseTemplate,
    basho_list: Vec<BashoInfo>,
}

pub fn basho_list(state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<BashoListTemplate>> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity)?;
    Ok(BashoListTemplate {
        base,
        basho_list: BashoInfo::list_all(&db)?,
    }.into())
}

#[derive(Template)]
#[template(path = "basho.html")]
struct BashoTemplate<'a> {
    base: BaseTemplate,
    basho: BashoInfo,
    leaders: Vec<BashoPlayerResults<'a>>,
    rikishi_by_rank: Vec<BashoRikishiByRank>,
    next_day: u8,
    initially_selectable: bool,
}

struct BashoPlayerResults<'a> {
    player: Player,
    total: u8,
    days: [Option<i8>; 15],
    picks: [Option<&'a BashoRikishi>; 5],
    rank: usize,
    is_self: bool,
}

#[derive(Clone)]
struct BashoRikishi {
    id: RikishiId,
    name: String,
    rank: Rank,
    results: [Option<bool>; 15],
    wins: u8,
    losses: u8,
    is_player_pick: bool,
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

struct BashoRikishiByRank {
    rank: String,
    rank_group: RankGroup,
    east: Option<BashoRikishi>,
    west: Option<BashoRikishi>,
}

impl BashoRikishiByRank {
    fn next_day(&self) -> u8 {
        max(self.east.as_ref().map_or(1, |r| r.next_day()),
            self.west.as_ref().map_or(1, |r| r.next_day()))
    }
}

pub fn basho(path: web::Path<BashoId>, state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity)?;
    let player_id = base.player.as_ref().map(|p| p.id);
    let picks = fetch_player_picks(&db, player_id, basho_id)?;
    let rikishi = fetch_rikishi(&db, basho_id, &picks)?;
    let basho = BashoInfo::with_id(&db, basho_id)?
            .ok_or_else(|| HandlerError::NotFound("basho".to_string()))?;
    let s = BashoTemplate {
        leaders: fetch_leaders(&db, basho_id, player_id, &rikishi.by_id)?,
        next_day: rikishi.by_rank.iter()
            .map(|rr| rr.next_day())
            .max()
            .unwrap_or(1),
        rikishi_by_rank: rikishi.by_rank,
        initially_selectable: !basho.has_started() && base.player.is_some() && picks.len() < RankGroup::count(),
        basho,
        base,
    }.render()?;
    Ok(HttpResponse::Ok().body(s))
}

fn fetch_player_picks(db: &Connection, player_id: Option<PlayerId>, basho_id: BashoId) -> Result<HashSet<RikishiId>> {
    let mut set = HashSet::with_capacity(5);
    if let Some(player_id) = player_id {
        debug!("fetching player {} picks for {}", player_id, basho_id);
        let mut stmt = db.prepare("
                SELECT
                    pick.rikishi_id
                FROM pick
                WHERE pick.player_id = ? AND pick.basho_id = ?
            ").unwrap();
        let rows = stmt.query_map(
                params![player_id, basho_id],
                |row| row.get(0)
            )?;
        for pick in rows {
            set.insert(pick?);
        }
    }
    debug!("player picks: {:?}", set);
    Ok(set)
}

fn fetch_leaders<'a>(db: &Connection, basho_id: BashoId, player_id: Option<PlayerId>, rikishi: &'a HashMap<RikishiId, BashoRikishi>)
    -> Result<Vec<BashoPlayerResults<'a>>> {

    const LIMIT: usize = 100;
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
            named_params!{
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
            ;
            let mut picks = [None; 5];
            let mut days = [None; 15];
            let mut total_validation = 0;
            for rikishi_id in picks_str.split(',').map(|id| id.parse().unwrap()) {
                if let Some(r) = rikishi.get(&rikishi_id) {
                    picks[*r.rank.group() as usize - 1] = Some(r);
                    for (day, win) in r.results.iter().enumerate() {
                        if let Some(true) = win {
                            days[day] = Some(days[day].unwrap_or(0) + 1);
                            total_validation += 1;
                        }
                    }
                }
            }
            assert_eq!(total, total_validation);
            BashoPlayerResults {
                is_self: player_id.map_or(false, |id| player.id == id),
                rank: 0, // populated later
                player, picks, total, days,
            }
        })
        .collect();
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
    Ok(leaders)
}

struct FetchedRikishiRow(Rank, RikishiId, String, Option<Day>, Option<bool>);

struct FetchRikishiResult {
    by_rank: Vec<BashoRikishiByRank>,
    by_id: HashMap<RikishiId, BashoRikishi>,
}

fn fetch_rikishi(db: &Connection, basho_id: BashoId, picks: &HashSet<RikishiId>)
    -> Result<FetchRikishiResult> {

    debug!("fetching rikishi results for basho {}", basho_id);
    let vec: Vec<BashoRikishiByRank> = db.prepare("
            SELECT
                banzuke.rank,
                banzuke.rikishi_id,
                banzuke.family_name,
                torikumi.day,
                torikumi.win
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
                    is_player_pick: picks.contains(&arow.1),
                };
                for FetchedRikishiRow(_, _, _, day, win) in rows {
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
    Ok(FetchRikishiResult {
        by_rank: vec,
        by_id: map,
    })
}


#[derive(Deserialize)]
#[serde(rename_all = "kebab-case")]
pub struct SavePicksFormData {
    rank_group_1: Option<RikishiId>,
    rank_group_2: Option<RikishiId>,
    rank_group_3: Option<RikishiId>,
    rank_group_4: Option<RikishiId>,
    rank_group_5: Option<RikishiId>,
}

pub fn save_picks(path: web::Path<BashoId>, form: web::Form<SavePicksFormData>, state: web::Data<AppState>, identity: Identity)
    -> Result<impl Responder> {

    let player_id = identity
        .identity()
        .ok_or(HandlerError::MustBeLoggedIn)?
        .parse()?;
    let picks = &[form.rank_group_1, form.rank_group_2, form.rank_group_3, form.rank_group_4, form.rank_group_5];
    let mut db = state.db.lock().unwrap();
    data::basho::save_player_picks(&mut db, player_id, path.into_inner(), *picks)
        .map_err(|e| e.into())
}
