extern crate itertools;
use actix_identity::Identity;
use askama_web::WebTemplate;
use rusqlite::Connection;
use std::collections::HashSet;

use super::{BaseTemplate, HandlerError, IdentityExt, Result};
use crate::data::heya::HeyaId;
use crate::data::leaders::{BashoPlayerResults, ResultPlayer};
use crate::data::{
    self, BashoId, BashoInfo, BashoRikishiByRank, DataError, FetchBashoRikishi, Heya, PlayerId,
    RankGroup, RankSide, RikishiId,
};
use crate::AppState;

use actix_web::{get, http, post, web, Either, HttpResponse, Responder};
use askama::Template;

#[derive(Template, WebTemplate)]
#[template(path = "basho.html")]
pub struct BashoTemplate {
    base: BaseTemplate,
    basho: BashoInfo,
    heya: Option<Heya>,
    leaders: Vec<BashoPlayerResults>,
    self_leader_index: Option<usize>,
    rikishi_by_rank: Vec<BashoRikishiByRank>,
    next_day: u8,
    initially_selectable: bool,
}

pub struct HeyaOption<'a> {
    heya: &'a Heya,
    selected: bool,
}

impl BashoTemplate {
    fn self_rank(&self) -> Option<usize> {
        if !self.basho.has_started() {
            return None;
        }

        self.leaders.iter().find_map(|l| match l.player {
            ResultPlayer::RankedPlayer(_, rank) if l.is_self => Some(rank),
            _ => None,
        })
    }

    fn heya_options(&self) -> Option<Vec<HeyaOption<'_>>> {
        let selected_heya_id = self.heya.as_ref().map_or(-1, |h| h.id);
        self.base.player.as_ref().map(|player| {
            player
                .heyas
                .as_ref()
                .unwrap()
                .iter()
                .map(|heya| HeyaOption {
                    heya,
                    selected: heya.id == selected_heya_id,
                })
                .collect()
        })
    }
}

#[derive(Deserialize)]
pub struct BashoQuery {
    all: Option<bool>,
    heya: Option<HeyaId>,
}

const DEFAULT_LEADERS_LIMIT: usize = 100;

#[get("")]
pub async fn basho(
    path: web::Path<BashoId>,
    query: web::Query<BashoQuery>,
    state: web::Data<AppState>,
    identity: Option<Identity>,
) -> Result<Either<BashoTemplate, HttpResponse>> {
    let basho_id = path.into_inner();
    let db = state.db.lock().unwrap();

    let basho = BashoInfo::with_id(&db, basho_id)?
        .ok_or_else(|| HandlerError::NotFound("basho".to_string()))?;
    if let Some(external_link) = basho.external_link {
        return Ok(Either::Right(
            HttpResponse::SeeOther()
                .insert_header((http::header::LOCATION, external_link))
                .finish(),
        ));
    }
    let base = BaseTemplate::new(&db, identity.as_ref(), &state)?;
    let player_id = base.player.as_ref().map(|p| p.id);
    let picks = fetch_player_picks(&db, player_id, basho_id)?;
    let FetchBashoRikishi {
        by_id: rikishi_by_id,
        by_rank: rikishi_by_rank,
    } = FetchBashoRikishi::with_db(&db, basho_id, &picks)?;
    let limit = if !basho.has_started() || query.all.unwrap_or(false) {
        1000000
    } else {
        DEFAULT_LEADERS_LIMIT
    };

    // Ignore heya param in pre-basho view since it duplicates info on the heya page
    let heya = if basho.has_started() {
        query
            .0
            .heya
            .map(|heya_id| Heya::with_id(&db, heya_id, false))
            .transpose()?
    } else {
        None
    };

    let leaders = BashoPlayerResults::fetch(
        &db,
        basho_id,
        player_id,
        rikishi_by_id,
        basho.has_started(),
        limit,
        heya.as_ref().map(|h| h.id),
    )?;
    let self_leader_index = leaders.iter().position(|l| l.is_self);
    Ok(Either::Left(BashoTemplate {
        leaders,
        self_leader_index,
        heya,
        next_day: rikishi_by_rank
            .iter()
            .map(|rr| rr.next_day())
            .max()
            .unwrap_or(1),
        rikishi_by_rank,
        initially_selectable: !basho.has_started()
            && base.player.is_some()
            && picks.len() < RankGroup::count(),
        basho,
        base,
    }))
}

fn fetch_player_picks(
    db: &Connection,
    player_id: Option<PlayerId>,
    basho_id: BashoId,
) -> Result<HashSet<RikishiId>> {
    let mut set = HashSet::with_capacity(5);
    if let Some(player_id) = player_id {
        debug!("fetching player {} picks for {}", player_id, basho_id);
        let mut stmt = db
            .prepare(
                "
                SELECT
                    pick.rikishi_id
                FROM pick
                WHERE pick.player_id = ? AND pick.basho_id = ?
            ",
            )
            .unwrap();
        let rows = stmt
            .query_map(params![player_id, basho_id], |row| row.get(0))
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

#[post("/picks")]
pub async fn save_picks(
    path: web::Path<BashoId>,
    form: web::Form<SavePicksFormData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    let player_id = identity.player_id()?;
    let picks = &[
        form.rank_group_1,
        form.rank_group_2,
        form.rank_group_3,
        form.rank_group_4,
        form.rank_group_5,
    ];
    let mut db = state.db.lock().unwrap();
    match data::basho::save_player_picks(&mut db, player_id, path.into_inner(), *picks) {
        Ok(_) => Ok(HttpResponse::Ok()),
        Err(e) => Err(e.into()),
    }
}
