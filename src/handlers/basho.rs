extern crate itertools;
use std::collections::HashSet;
use actix_identity::Identity;
use rusqlite::Connection;

use super::{BaseTemplate, Result, HandlerError, IdentityExt};
use crate::data::{self, RankGroup, BashoId, BashoInfo, BashoRikishiByRank, FetchBashoRikishi, PlayerId, RikishiId, DataError};
use crate::data::leaders::{BashoPlayerResults, ResultPlayer};
use crate::AppState;

use actix_web::{web, http, HttpResponse, Responder, Either};
use askama::Template;

#[derive(Template)]
#[template(path = "basho.html")]
pub struct BashoTemplate {
    base: BaseTemplate,
    basho: BashoInfo,
    leaders: Vec<BashoPlayerResults>,
    rikishi_by_rank: Vec<BashoRikishiByRank>,
    next_day: u8,
    initially_selectable: bool,
}

pub async fn basho(path: web::Path<BashoId>, state: web::Data<AppState>, identity: Identity)
    -> Result<Either<BashoTemplate, HttpResponse>> {

    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let basho = BashoInfo::with_id(&db, basho_id)?
            .ok_or_else(|| HandlerError::NotFound("basho".to_string()))?;
    if let Some(external_link) = basho.external_link {
        return Ok(
            Either::B(
                HttpResponse::SeeOther()
                .header(http::header::LOCATION, external_link)
                .finish()
            )
        );
    }
    let base = BaseTemplate::new(&db, &identity)?;
    let player_id = base.player.as_ref().map(|p| p.id);
    let picks = fetch_player_picks(&db, player_id, basho_id)?;
    let FetchBashoRikishi {by_id: rikishi_by_id, by_rank: rikishi_by_rank} = FetchBashoRikishi::with_db(&db, basho_id, &picks)?;
    let limit = if state.config.is_dev() {3} else {500};
    Ok(Either::A(BashoTemplate {
        leaders: BashoPlayerResults::fetch(&db, basho_id, player_id, rikishi_by_id, basho.has_started(), limit)?,
        next_day: rikishi_by_rank.iter()
            .map(|rr| rr.next_day())
            .max()
            .unwrap_or(1),
        rikishi_by_rank,
        initially_selectable: !basho.has_started() && base.player.is_some() && picks.len() < RankGroup::count(),
        basho,
        base,
    }))
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
            )
            .map_err(DataError::from)?;
        for pick in rows {
            set.insert(pick.unwrap());
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

pub async fn save_picks(path: web::Path<BashoId>, form: web::Form<SavePicksFormData>, state: web::Data<AppState>, identity: Identity)
    -> Result<impl Responder> {

    let player_id = identity.require_player_id()?;
    let picks = &[form.rank_group_1, form.rank_group_2, form.rank_group_3, form.rank_group_4, form.rank_group_5];
    let mut db = state.db.lock().unwrap();
    match data::basho::save_player_picks(&mut db, player_id, path.into_inner(), *picks) {
        Ok(_) => Ok(HttpResponse::Ok()),
        Err(e) => Err(e.into())
    }
}
