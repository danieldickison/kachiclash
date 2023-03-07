use actix_identity::Identity;
use actix_web::{web, Either, HttpResponse};
use askama::Template;
use reqwest::header;

use super::{BaseTemplate, HandlerError, Result};
use crate::data::player::{self, Player};
use crate::handlers::IdentityExt;
use crate::AppState;

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

pub async fn settings_page(
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<SettingsTemplate> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity, &state)?;
    if base.player.is_some() {
        Ok(SettingsTemplate {
            base,
            message: None,
            error: None,
        })
    } else {
        Err(HandlerError::MustBeLoggedIn)
    }
}

pub async fn settings_post(
    form: web::Form<FormData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<Either<SettingsTemplate, HttpResponse>> {
    let player_id = identity.require_player_id()?;
    let db = state.db.lock().unwrap();

    if !player::name_is_valid(&form.name) {
        return Ok(Either::Left(SettingsTemplate {
            base: BaseTemplate::new(&db, &identity, &state)?,
            message: None,
            error: Some(format!("Invalid name: {}", form.name)),
        }));
    }

    match db.execute(
        "UPDATE player SET name = ? WHERE id = ?",
        params![form.name, player_id],
    ) {
        Ok(_) => Ok(Either::Right(
            HttpResponse::SeeOther()
                .insert_header((header::LOCATION, Player::url_path_for_name(&form.name)))
                .finish(),
        )),
        Err(e) => Ok(Either::Left(SettingsTemplate {
            base: BaseTemplate::new(&db, &identity, &state)?,
            message: None,
            error: Some(format!("Error: {}", e)),
        })),
    }
}
