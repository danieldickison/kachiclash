extern crate itertools;
use std::collections::HashSet;
use actix_identity::Identity;
use rusqlite::Connection;

use super::{BaseTemplate, Result, HandlerError};
use crate::data::{self, RankGroup, BashoId, BashoInfo, BashoRikishiByRank, FetchBashoRikishi, PlayerId, RikishiId};
use crate::data::leaders::{BashoPlayerResults, ResultPlayer};
use crate::AppState;

use actix_web::{web, HttpResponse, Responder};
use askama::Template;


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

pub fn basho(path: web::Path<BashoId>, state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity)?;
    let player_id = base.player.as_ref().map(|p| p.id);
    let picks = fetch_player_picks(&db, player_id, basho_id)?;
    let rikishi = FetchBashoRikishi::with_db(&db, basho_id, &picks)?;
    let basho = BashoInfo::with_id(&db, basho_id)?
            .ok_or_else(|| HandlerError::NotFound("basho".to_string()))?;
    let s = BashoTemplate {
        leaders: BashoPlayerResults::fetch(&db, basho_id, player_id, &rikishi.by_id)?,
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
