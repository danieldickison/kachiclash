use askama::Template;
use actix_web::web;
use actix_identity::Identity;

use super::{Result, BaseTemplate, HandlerError};
use crate::data::{PlayerId, Player, player::BashoScore};
use crate::AppState;

#[derive(Template)]
#[template(path = "player.html")]
pub struct PlayerTemplate {
    base: BaseTemplate,
    player: Player,
    basho_scores: Vec<BashoScore>,
}

pub async fn player(path: web::Path<PlayerId>, state: web::Data<AppState>, identity: Identity)
    -> Result<PlayerTemplate> {

    let player_id = path.into_inner();
    let db = state.db.lock().unwrap();
    let player = Player::with_id(&db, player_id)?
        .ok_or_else(|| HandlerError::NotFound("player".to_string()))?;
    let base = BaseTemplate::new(&db, &identity)?;
    let basho_scores = BashoScore::with_player_id(&db, player_id)?;
    Ok(PlayerTemplate {
        base,
        player,
        basho_scores,
    })
}
