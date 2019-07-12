extern crate itertools;
use std::collections::HashSet;
use actix_identity::Identity;
use itertools::Itertools;
use rusqlite::{Connection, Result as SqlResult};

use super::{BaseTemplate, Result, HandlerError};
use crate::data::{Rank, RankSide, RankGroup, BashoId, BashoInfo, PlayerId, RikishiId};
use crate::AppState;

use actix_web::{web, HttpResponse, Responder};
use askama::Template;

pub fn basho_list(_state: web::Data<AppState>) -> impl Responder {

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
        basho: BashoInfo::with_id(&db, basho_id)?.ok_or(HandlerError::NotFound("basho".to_string()))?,
        base: base,
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
            JOIN torikumi ON torikumi.rikishi_id = pick.rikishi_id AND torikumi.basho_id = pick.basho_id
            WHERE player.id IN (
                SELECT pick.player_id
                FROM pick
                JOIN torikumi ON torikumi.rikishi_id = pick.rikishi_id AND torikumi.basho_id = pick.basho_id
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
            |row| -> SqlResult<(PlayerId, String, u8, i8)> {
                Ok((
                    row.get("id")?,
                    row.get("name")?,
                    row.get("day")?,
                    row.get("wins")?,
                ))
            }
        )?
        .collect::<SqlResult<Vec<(PlayerId, String, u8, i8)>>>()?
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
                results.days[day as usize - 1] = Some(wins);
                results.total += wins;
            }
            results
        })
        .into_iter()
        .sorted_by_key(|result| -result.total)
        .collect()
    )
}

fn fetch_rikishi(db: &Connection, basho_id: BashoId, picks: HashSet<RikishiId>) -> Result<Vec<BashoRikishiByRank>> {
    debug!("fetching rikishi results for basho {}", basho_id);
    Ok(db.prepare("
            SELECT
                rikishi_basho.rank,
                rikishi_basho.rikishi_id,
                rikishi_basho.family_name,
                torikumi.day,
                torikumi.win
            FROM rikishi_basho
            NATURAL JOIN torikumi
            WHERE
                rikishi_basho.basho_id = ?
            ORDER BY rikishi_basho.rikishi_id, torikumi.day
        ").unwrap()
        .query_map(
            params![basho_id],
            |row| -> SqlResult<(Rank, RikishiId, String, u8, Option<bool>)> {
                Ok((
                    row.get("rank")?,
                    row.get("rikishi_id")?,
                    row.get("family_name")?,
                    row.get("day")?,
                    row.get("win")?,
                ))
            }
        )?
        .collect::<SqlResult<Vec<(Rank, RikishiId, String, u8, Option<bool>)>>>()?
        .into_iter()
        .group_by(|row| (row.0.name, row.0.number)) // rank name and number but group east/west together
        .into_iter()
        .sorted_by(|(rank1, _), (rank2, _)| rank1.cmp(rank2))
        .into_iter()
        .map(|(rank, pair)| {
            let mut out = BashoRikishiByRank {
                rank: format!("{:}{}", rank.0, rank.1),
                rank_group: RankGroup::for_rank(rank.0, rank.1),
                east: None,
                west: None,
            };
            for (_, rows) in &pair.into_iter().group_by(|row| row.0) {
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
                for (_, _, _, day, win) in rows {
                    match win {
                        Some(true) => rikishi.wins += 1,
                        Some(false) => rikishi.losses += 1,
                        None => ()
                    }
                    rikishi.results[day as usize - 1] = win
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
