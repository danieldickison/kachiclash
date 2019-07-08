extern crate itertools;
use actix_identity::Identity;
use itertools::Itertools;
use rusqlite::{Connection, Result as SqlResult};

use super::{AppState, BaseTemplate, Result};
use super::data::{Rank, RankSide};

use actix_web::{web, HttpResponse, Responder};
use askama::Template;

pub fn basho_list(_state: web::Data<AppState>) -> impl Responder {

}

#[derive(Template)]
#[template(path = "basho.html")]
struct BashoTemplate {
    base: BaseTemplate,
    leaders: Vec<BashoPlayerResults>,
    rikishi_by_rank: Vec<BashoRikishiByRank>,
}

struct BashoPlayerResults {
    name: String,
    total: u8,
    days: [Option<u8>; 15],
}

struct BashoRikishiByRank {
    rank: String,
    has_east: bool,
    east_name: String,
    east_results: [Option<bool>; 15],
    has_west: bool,
    west_name: String,
    west_results: [Option<bool>; 15],
}

pub fn basho(path: web::Path<u32>, state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let base = BaseTemplate::new(&state, &identity)?;
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let s = BashoTemplate {
        base: base,
        leaders: fetch_leaders(&db, basho_id)?,
        rikishi_by_rank: fetch_rikishi(&db, basho_id)?,
    }.render()?;
    Ok(HttpResponse::Ok().content_type("text/html; charset=utf-8").body(s))
}

fn fetch_leaders(db: &Connection, basho_id: u32) -> Result<Vec<BashoPlayerResults>> {
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
            |row| -> SqlResult<(u32, String, u8, u8)> {
                Ok((
                    row.get("id")?,
                    row.get("name")?,
                    row.get("day")?,
                    row.get("wins")?,
                ))
            }
        )?
        .collect::<SqlResult<Vec<(u32, String, u8, u8)>>>()?
        .into_iter()
        .group_by(|row| row.0)
        .into_iter()
        .map(|(_player_id, rows)| {
            let mut rows = rows.peekable();
            let name = rows.peek().unwrap().1.to_string();
            let mut days: [Option<u8>; 15] = [None; 15];
            let mut total = 0;
            for (_, _, day, wins) in rows {
                days[day as usize - 1] = Some(wins);
                total += wins;
            }
            BashoPlayerResults {
                name: name,
                total: total,
                days: days
            }
        })
        .into_iter()
        .sorted_by_key(|result| result.total)
        .collect()
    )
}

fn fetch_rikishi(db: &Connection, basho_id: u32) -> Result<Vec<BashoRikishiByRank>> {
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
            |row| -> SqlResult<(Rank, u32, String, u8, Option<bool>)> {
                Ok((
                    row.get("rank")?,
                    row.get("rikishi_id")?,
                    row.get("family_name")?,
                    row.get("day")?,
                    row.get("win")?,
                ))
            }
        )?
        .collect::<SqlResult<Vec<(Rank, u32, String, u8, Option<bool>)>>>()?
        .into_iter()
        .group_by(|row| (row.0.name, row.0.number)) // rank name and number but group east/west together
        .into_iter()
        .sorted_by(|(rank1, _), (rank2, _)| rank1.cmp(rank2))
        .into_iter()
        .map(|(rank, pair)| {
            let mut out = BashoRikishiByRank {
                rank: format!("{:}{}", rank.0, rank.1),
                has_east: false,
                east_name: "".to_string(),
                east_results: [None; 15],
                has_west: false,
                west_name: "".to_string(),
                west_results: [None; 15],
            };
            for (_, rows) in &pair.into_iter().group_by(|row| row.0) {
                let mut rows = rows.peekable();
                let arow = rows.peek().unwrap();
                let name = arow.2.to_string();
                match arow.0.side {
                    RankSide::East => {
                        out.has_east = true;
                        out.east_name = name;
                        for (_, _, _, day, win) in rows {
                            out.east_results[day as usize - 1] = win
                        }
                    }
                    RankSide::West => {
                        out.has_west = true;
                        out.west_name = name;
                        for (_, _, _, day, win) in rows {
                            out.west_results[day as usize - 1] = win
                        }
                    }
                }
            }
            out
        })
        .collect()
    )
}
