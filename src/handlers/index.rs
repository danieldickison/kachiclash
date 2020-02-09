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
    leader_basho_count: usize,
    basho_list: Vec<BashoInfo>,
    next_basho_id: BashoId,
}

#[derive(Deserialize)]
pub struct QueryParams {
    b: Option<usize>
}

pub async fn index(query: web::Query<QueryParams>, state: web::Data<AppState>, identity: Identity) -> Result<IndexTemplate> {
    let leader_basho_count = query.b.unwrap_or(6);
    let db = state.db.lock().unwrap();
    let basho_list = BashoInfo::list_all(&db)?;
    let current_basho_id = basho_list.first().map(|b| b.id);
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders: HistoricLeader::with_first_basho(&db, nth_completed_basho_id(&basho_list, leader_basho_count - 1), 100)?,
        leader_basho_count,
        basho_list,
        next_basho_id: current_basho_id
            .map(|id| id.next_honbasho())
            .unwrap_or_else(|| "201911".parse().unwrap()),
    })
}

fn nth_completed_basho_id(basho_list: &Vec<BashoInfo>, n: usize) -> Option<BashoId> {
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
