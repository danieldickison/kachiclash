extern crate itertools;
use itertools::Itertools;

use rusqlite::{Connection};

use super::AppState;

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
    east_name: String,
    east_results: [Option<bool>; 15],
    west_name: String,
    west_results: [Option<bool>; 15],
}

pub fn basho(path: web::Path<u32>, state: web::Data<AppState>, _session: Session) -> impl Responder {
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let s = BashoTemplate {
        leaders: fetch_leaders(&db, basho_id),
        rikishi_by_rank: vec![BashoRikishiByRank {
            rank: "M1".to_string(),
            east_name: "foo".to_string(),
            east_results: [None; 15],
            west_name: "bar".to_string(),
            west_results: [Some(true); 15],
        }]
    }.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

fn fetch_leaders(db: &Connection, basho_id: u32) -> Vec<BashoPlayerResults> {
    db
        .prepare("
            SELECT
                player.id,
                player.name,
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
            ORDER BY player.id, torikumi.day
        ").unwrap()
        .query_map_named(
            named_params!{
                ":basho_id": basho_id
            },
            |row| -> Result<(u32, String, u8), _> { Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
            ))}
        )
        .and_then(|mapped_rows| mapped_rows.collect::<Result<Vec<(u32, String, u8)>, _>>())
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
            for (i, (_, _, wins)) in rows.enumerate() {
                days[i] = Some(wins);
                total += wins;
            }
            BashoPlayerResults {
                name: name,
                total: total,
                days: days
            }
        })
        .collect::<Vec<BashoPlayerResults>>()
}
