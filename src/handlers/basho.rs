extern crate itertools;
use std::collections::HashSet;
use actix_identity::Identity;
use itertools::Itertools;
use rusqlite::{Connection, Result as SqlResult};

use super::{BaseTemplate, Result, HandlerError, AskamaResponder};
use crate::data::{self, Rank, RankSide, RankGroup, BashoId, BashoInfo, PlayerId, RikishiId, Day};
use crate::AppState;

use actix_web::{web, HttpResponse, Responder};
use askama::Template;


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
struct BashoTemplate {
    base: BaseTemplate,
    basho: BashoInfo,
    leaders: Vec<BashoPlayerResults>,
    rikishi_by_rank: Vec<BashoRikishiByRank>,
}

struct BashoPlayerResults {
    id: PlayerId,
    name: String,
    total: i8,
    days: [Option<i8>; 15],
}

struct BashoRikishi {
    id: RikishiId,
    name: String,
    rank: Rank,
    results: [Option<bool>; 15],
    wins: u8,
    losses: u8,
    is_player_pick: bool,
}

struct BashoRikishiByRank {
    rank: String,
    rank_group: RankGroup,
    east: Option<BashoRikishi>,
    west: Option<BashoRikishi>,
}

pub fn basho(path: web::Path<BashoId>, state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity)?;
    let player_id = base.player.as_ref().map(|p| p.id);
    let picks = fetch_player_picks(&db, player_id, basho_id)?;
    let s = BashoTemplate {
        basho: BashoInfo::with_id(&db, basho_id)?
            .ok_or_else(|| HandlerError::NotFound("basho".to_string()))?,
        base,
        leaders: fetch_leaders(&db, basho_id)?,
        rikishi_by_rank: fetch_rikishi(&db, basho_id, picks)?,
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

fn fetch_leaders(db: &Connection, basho_id: BashoId) -> Result<Vec<BashoPlayerResults>> {
    debug!("fetching leaders for basho {}", basho_id);
    Ok(db.prepare("
            SELECT
                player.id,
                player.name,
                torikumi.day,
                SUM(torikumi.win) AS wins
            FROM player
            JOIN pick ON pick.player_id = player.id AND pick.basho_id = :basho_id
            LEFT JOIN torikumi ON torikumi.rikishi_id = pick.rikishi_id AND torikumi.basho_id = pick.basho_id
            WHERE player.id IN (
                SELECT pick.player_id
                FROM pick
                LEFT JOIN torikumi ON torikumi.rikishi_id = pick.rikishi_id AND torikumi.basho_id = pick.basho_id
                WHERE pick.basho_id = :basho_id
                GROUP BY pick.player_id
                ORDER BY SUM(torikumi.win) DESC
                LIMIT 10
            )
            GROUP BY player.id, torikumi.day
            ORDER BY player.id, torikumi.day
        ").unwrap()
        .query_map_named(
            named_params!{
                ":basho_id": basho_id
            },
            |row| -> SqlResult<(PlayerId, String, Option<u8>, Option<i8>)> {
                Ok((
                    row.get("id")?,
                    row.get("name")?,
                    row.get("day")?,
                    row.get("wins")?,
                ))
            }
        )?
        .collect::<SqlResult<Vec<(PlayerId, String, Option<u8>, Option<i8>)>>>()?
        .into_iter()
        .group_by(|row| row.0)
        .into_iter()
        .map(|(_player_id, rows)| {
            let mut rows = rows.peekable();
            let arow = rows.peek().unwrap();
            let mut results = BashoPlayerResults {
                id: arow.0,
                name: arow.1.to_string(),
                total: 0,
                days: [None; 15]
            };
            for (_, _, day, wins) in rows {
                if let Some(day) = day {
                    results.days[day as usize - 1] = wins;
                    results.total += wins.unwrap_or(0);
                }
            }
            results
        })
        .sorted_by_key(|result| -result.total)
        .collect()
    )
}

struct FetchedRikishiRow(Rank, RikishiId, String, Option<Day>, Option<bool>);

fn fetch_rikishi(db: &Connection, basho_id: BashoId, picks: HashSet<RikishiId>) -> Result<Vec<BashoRikishiByRank>> {
    debug!("fetching rikishi results for basho {}", basho_id);
    Ok(db.prepare("
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
        .collect()
    )
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
