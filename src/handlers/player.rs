use actix_identity::Identity;
use actix_web::{get, web};
use askama::Template;

use super::{BaseTemplate, HandlerError, Result};
use crate::data::Heya;
use crate::data::{player::BashoScore, Player};
use crate::handlers::IdentityExt;
use crate::AppState;

#[derive(Template)]
#[template(path = "player.html")]
pub struct PlayerTemplate {
    base: BaseTemplate,
    player: Player,
    basho_scores: Vec<BashoScore>,
    recruit_heyas: Vec<Heya>,
}

#[get("/player/{player}")]
pub async fn player_page(
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

    let recruit_heyas = identity
        .as_ref()
        .and_then(|i| i.player_id().ok())
        .map(|user_player_id| Heya::for_player(&db, user_player_id))
        .transpose()?
        .unwrap_or_default()
        .into_iter()
        .filter(|heya| {
            heya.oyakata.id == identity.as_ref().unwrap().player_id().unwrap()
        })
        .filter(|hosted_heya| {
            !player
                .heyas
                .as_ref()
                .unwrap()
                .iter()
                .any(|member_heya| member_heya.id == hosted_heya.id)
        })
        .collect();

    Ok(PlayerTemplate {
        base,
        player,
        basho_scores,
        recruit_heyas,
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
