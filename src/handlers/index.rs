use crate::{data, AppState};
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
}

pub fn index(state: Data<AppState>, identity: Identity) -> Result<AskamaResponder<IndexTemplate>> {
    let db = state.db.lock().unwrap();
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders: data::player::list_players(&db),
        basho_list: data::BashoInfo::list_all(&db)?,
    }.into())
}
