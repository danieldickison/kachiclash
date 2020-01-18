use actix_web::web;
use actix_identity::Identity;
use askama::Template;

use crate::AppState;
use super::{BaseTemplate, Result, HandlerError};
use crate::data::player;
use crate::handlers::IdentityExt;


#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    base: BaseTemplate,
    message: Option<String>,
    error: Option<String>,
}

#[derive(Deserialize)]
pub struct FormData {
    name: String,
}

pub async fn settings_page(state: web::Data<AppState>, identity: Identity) -> Result<SettingsTemplate> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity)?;
    if base.player.is_some() {
        Ok(SettingsTemplate {
            base,
            message: None,
            error: None,
        }.into())
    } else {
        Err(HandlerError::MustBeLoggedIn.into())
    }
}

pub async fn settings_post(form: web::Form<FormData>, state: web::Data<AppState>, identity: Identity) -> Result<SettingsTemplate> {

    let player_id = identity.require_player_id()?;
    let db = state.db.lock().unwrap();

    if !player::name_is_valid(&form.name) {
        return Ok(SettingsTemplate {
            base: BaseTemplate::new(&db, &identity)?,
            message: None,
            error: Some(format!("Invalid name: {}", form.name)),
        }.into());
    }

    let result = db.execute(
        "UPDATE player SET name = ? WHERE id = ?",
        params![form.name, player_id]
    );

    let base = BaseTemplate::new(&db, &identity)?;
    if base.player.is_some() {
        Ok(SettingsTemplate {
            base,
            message: match result {
                Ok(_) => Some("Display name saved".to_string()),
                Err(_) => None,
            },
            error: match result {
                Ok(_) => None,
                Err(e) => Some(format!("Error: {}", e)),
            },
        }.into())
    } else {
        Err(HandlerError::MustBeLoggedIn.into())
    }
}
