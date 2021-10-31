use askama::Template;
use actix_web::web;
use actix_identity::Identity;

use super::{Result, BaseTemplate, HandlerError};
use crate::data::{player, Player, player::BashoScore};
use crate::AppState;

#[derive(Template)]
#[template(path = "player.html")]
pub struct PlayerTemplate {
    base: BaseTemplate,
    player: Player,
    basho_scores: Vec<BashoScore>,
}

pub async fn player(path: web::Path<String>, state: web::Data<AppState>, identity: Identity)
    -> Result<PlayerTemplate> {

    let name = path.into_inner();
    let db = state.db.lock().unwrap();
    let player = Player::with_name(&db, name)?
        .ok_or_else(|| HandlerError::NotFound("player".to_string()))?;
    let basho_scores = BashoScore::with_player_id(&db, player.id, &player.name)?;
    let base = BaseTemplate::new(&db, &identity)?;
    Ok(PlayerTemplate {
        base,
        player,
        basho_scores,
    })
}

impl PlayerTemplate {
    fn is_self(&self) -> bool {
        self.base.player.as_ref()
        .map_or(false, |p| p.id == self.player.id)
    }
}
