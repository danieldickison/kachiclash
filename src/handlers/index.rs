use crate::AppState;
use crate::data::{self, BashoId, BashoInfo, Player};
use super::{BaseTemplate, Result};
use super::askama_responder::AskamaResponder;
use actix_web::web::Data;
use actix_identity::Identity;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplate,
    leaders: Vec<data::player::Player>,
    basho_list: Vec<data::BashoInfo>,
    next_basho_id: BashoId,
}

pub fn index(state: Data<AppState>, identity: Identity) -> Result<AskamaResponder<IndexTemplate>> {
    let db = state.db.lock().unwrap();
    let basho_list = BashoInfo::list_all(&db)?;
    let current_basho_id = basho_list.first().map(|b| b.id);
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders: Player::list_all(&db)?,
        basho_list,
        next_basho_id: current_basho_id.map(|id| id.next_honbasho()).unwrap_or("201911".parse().unwrap()),
    }.into())
}
