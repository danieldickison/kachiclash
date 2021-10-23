use crate::AppState;
use crate::data::{BashoId, BashoInfo};
use crate::data::leaders::HistoricLeader;
use super::{BaseTemplate, Result};
use actix_web::web;
use actix_identity::Identity;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplate,
    leaders: Vec<HistoricLeader>,
    current_basho: Option<BashoInfo>,
    prev_basho: Option<BashoInfo>,
    next_basho_id: BashoId,
}

const LEADER_BASHO_COUNT_OPTIONS: [usize; 3] = [6, 3, 2];

pub async fn index(state: web::Data<AppState>, identity: Identity) -> Result<IndexTemplate> {
    let db = state.db.lock().unwrap();
    let (current_basho, prev_basho) = BashoInfo::current_and_previous(&db)?;
    let next_basho_id =
        current_basho.as_ref().or(prev_basho.as_ref())
        .map(|basho| basho.id.next())
        .unwrap_or_else(|| "201911".parse().unwrap());
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders: HistoricLeader::with_first_basho(&db, prev_basho.as_ref().map(|basho| basho.id.incr(-5)), 270)?,
        current_basho,
        prev_basho,
        next_basho_id
    })
}

fn nth_completed_basho_id(basho_list: &[BashoInfo], n: usize) -> Option<BashoId> {
    if basho_list.is_empty() { return None; }

    let mut n = n;
    if basho_list.first().unwrap().winners.is_empty() {
        n += 1;
    }
    if n >= basho_list.len() {
        n = basho_list.len() - 1;
    }
    basho_list.get(n).map(|b| b.id)
}
