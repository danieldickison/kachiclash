use crate::AppState;
use crate::data::{BashoId, BashoInfo, Player};
use super::{BaseTemplate, Result};
use actix_web::web::Data;
use actix_identity::Identity;
use askama::Template;

#[derive(Template)]
#[template(path = "index.html")]
pub struct IndexTemplate {
    base: BaseTemplate,
    leaders: Vec<Player>,
    basho_list: Vec<BashoInfo>,
    next_basho_id: BashoId,
}

pub async fn index(state: Data<AppState>, identity: Identity) -> Result<IndexTemplate> {
    let db = state.db.lock().unwrap();
    let basho_list = BashoInfo::list_all(&db)?;
    let current_basho_id = basho_list.first().map(|b| b.id);
    Ok(IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders: Player::list_all(&db)?,
        basho_list,
        next_basho_id: current_basho_id
            .map(|id| id.next_honbasho())
            .unwrap_or_else(|| "201911".parse().unwrap()),
    }.into())
}
