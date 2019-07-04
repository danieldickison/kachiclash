extern crate itertools;
use itertools::Itertools;

use rusqlite::{Connection};

use super::AppState;
use super::data::{Rank, RankSide};

use actix_web::{web, HttpResponse, Responder};
use actix_session::Session;
use askama::Template;

#[derive(Template)]
#[template(path = "basho.html")]
struct BashoTemplate {
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
    east_name: Option<String>,
    east_results: [Option<bool>; 15],
    west_name: Option<String>,
    west_results: [Option<bool>; 15],
}

pub fn basho(path: web::Path<u32>, state: web::Data<AppState>, _session: Session) -> impl Responder {
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let s = BashoTemplate {
        leaders: fetch_leaders(&db, basho_id),
        rikishi_by_rank: fetch_rikishi(&db, basho_id),
    }.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

fn fetch_leaders(db: &Connection, basho_id: u32) -> Vec<BashoPlayerResults> {
    debug!("fetching leadings for basho {}", basho_id);
    db
        .prepare("
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
            |row| -> Result<(u32, String, u8, u8), _> { Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
            ))}
        )
        .and_then(|mapped_rows| mapped_rows.collect::<Result<Vec<(u32, String, u8, u8)>, _>>())
        .unwrap_or_else(|e| {
            warn!("failed to fetch leaderboard: {:?}", e);
            vec![]
        })
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
        .collect()
}

fn fetch_rikishi(db: &Connection, basho_id: u32) -> Vec<BashoRikishiByRank> {
    debug!("fetching rikishi results for basho {}", basho_id);
    db
        .prepare("
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
            ORDER BY rikishi_basho.rank, torikumi.day
        ").unwrap()
        .query_map(
            params![basho_id],
            |row| -> Result<(Rank, u32, String, u8, Option<bool>), _> { Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
            ))}
        )
        .and_then(|mapped_rows| mapped_rows.collect::<Result<Vec<(Rank, u32, String, u8, Option<bool>)>, _>>())
        .unwrap_or_else(|e| {
            warn!("failed to fetch rikishi: {:?}", e);
            vec![]
        })
        .into_iter()
        .group_by(|row| (row.0.name, row.0.number)) // rank name and number but group east/west together
        .into_iter()
        .map(|(rank, pair)| {
            let mut out = BashoRikishiByRank {
                rank: format!("{:}{}", rank.0, rank.1),
                east_name: None,
                east_results: [None; 15],
                west_name: None,
                west_results: [None; 15],
            };
            for (_, rows) in &pair.into_iter().group_by(|row| row.0) {
                let mut rows = rows.peekable();
                let arow = rows.peek().unwrap();
                let name = arow.2.to_string();

                // let mut total = 0;
                // for (_, _, day, wins) in rows {
                //     days[day as usize - 1] = Some(wins);
                //     total += wins;
                // }
                // todo tally results
                match arow.0.side {
                    RankSide::East => {
                        out.east_name = Some(name)
                    }
                    RankSide::West => {
                        out.west_name = Some(name)
                    }
                }
            }
            out
        })
        .collect()
}
