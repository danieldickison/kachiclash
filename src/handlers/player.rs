use actix_identity::Identity;
use actix_web::web;
use askama::Template;

use super::{BaseTemplate, HandlerError, Result};
use crate::data::{player::BashoScore, Player};
use crate::AppState;

#[derive(Template)]
#[template(path = "player.html")]
pub struct PlayerTemplate {
    base: BaseTemplate,
    player: Player,
    basho_scores: Vec<BashoScore>,
}

pub async fn player(
    path: web::Path<String>,
    state: web::Data<AppState>,
    identity: Option<Identity>,
) -> Result<PlayerTemplate> {
    let name = path.into_inner();
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, identity.as_ref(), &state)?;
    let player = Player::with_name(&db, name, base.current_or_next_basho_id)?
        .ok_or_else(|| HandlerError::NotFound("player".to_string()))?;
    let basho_scores = BashoScore::with_player_id(&db, player.id, &player.name)?;
    Ok(PlayerTemplate {
        base,
        player,
        basho_scores,
    })
}

impl PlayerTemplate {
    fn is_self(&self) -> bool {
        self.base
            .player
            .as_ref()
            .map_or(false, |p| p.id == self.player.id)
    }
}
