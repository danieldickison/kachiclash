use actix_web::web;
use actix_identity::Identity;
use askama::Template;

use crate::AppState;
use super::askama_responder::AskamaResponder;
use super::{BaseTemplate, Result, HandlerError};
use crate::data::{PlayerId, player};


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

pub fn settings_page(state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<SettingsTemplate>> {
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

pub fn settings_post(form: web::Form<FormData>, state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<SettingsTemplate>> {

    let player_id: PlayerId = match identity.identity() {
        Some(id) => match id.parse() {
            Ok(player_id) => player_id,
            Err(_) => return Err(HandlerError::MustBeLoggedIn.into()),
        },
        None => return Err(HandlerError::MustBeLoggedIn.into()),
    };

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
