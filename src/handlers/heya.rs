use actix_identity::Identity;
use actix_web::{get, http, post, web, HttpResponse, Responder};
use askama::Template;
use rusqlite::Connection;

use crate::data::heya::{HOST_MAX, JOIN_MAX};
use crate::data::{Heya, PlayerId};
use crate::handlers::{HandlerError, IdentityExt};
use crate::AppState;

use super::{BaseTemplate, Result};

#[derive(Template)]
#[template(path = "heya.html")]
pub struct HeyaTemplate {
    base: BaseTemplate,
    heya: Heya,
    is_oyakata: bool,
}

#[get("")]
pub async fn page(
    state: web::Data<AppState>,
    identity: Option<Identity>,
    path: web::Path<String>,
) -> Result<HeyaTemplate> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, identity.as_ref(), &state)?;
    let player_id = identity.and_then(|i| i.player_id().ok());
    let mut heya = Heya::with_slug(&db, &path, true)?;
    for m in heya.members.as_mut().unwrap() {
        m.is_self = player_id.is_some_and(|id| id == m.player.id);
    }
    Ok(HeyaTemplate {
        is_oyakata: player_id.map_or(false, |pid| pid == heya.oyakata.id),
        base,
        heya,
    })
}

#[derive(Debug, Deserialize)]
pub struct EditData {
    set_name: Option<String>,
    add_player_id: Option<PlayerId>,
    delete_player_id: Option<PlayerId>,
}

#[post("")]
pub async fn edit(
    path: web::Path<String>,
    data: web::Form<EditData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    let mut db = state.db.lock().unwrap();
    let mut heya = Heya::with_slug(&db, &path, false)?;
    apply_edit_actions(&mut heya, &mut db, data.0, identity.player_id()?)?;

    let updated_heya = Heya::with_id(&db, heya.id, false)?;
    Ok(HttpResponse::SeeOther()
        .insert_header((http::header::LOCATION, updated_heya.url_path()))
        .finish())
}

fn apply_edit_actions(
    heya: &mut Heya,
    db: &mut Connection,
    data: EditData,
    user: PlayerId,
) -> Result<()> {
    if let Some(name) = data.set_name {
        heya.set_name(db, &name)?;
    }
    if let Some(player_id) = data.add_player_id {
        if heya.oyakata.id == user {
            heya.add_member(db, player_id)?;
        } else {
            return Err(HandlerError::MustBeLoggedIn);
        }
    }
    if let Some(player_id) = data.delete_player_id {
        // Member can choose to leave; oyakata can kick others out:
        if heya.oyakata.id == user || player_id == user {
            heya.delete_member(db, player_id)?;
        } else {
            return Err(HandlerError::MustBeLoggedIn);
        }
    }

    Ok(())
}

#[derive(Template)]
#[template(path = "heya_list.html")]
pub struct HeyaListTemplate {
    base: BaseTemplate,
    heyas: Vec<Heya>,
    hosted: usize,
}

#[get("/heya")]
pub async fn list(
    state: web::Data<AppState>,
    identity: Option<Identity>,
) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    let heyas = Heya::list_all(&db)?;
    let player_id = identity.as_ref().and_then(|i| i.player_id().ok());
    Ok(HeyaListTemplate {
        base: BaseTemplate::new(&db, identity.as_ref(), &state)?,
        hosted: heyas
            .iter()
            .filter(|h| h.oyakata.id == player_id.unwrap_or(-1))
            .count(),
        heyas,
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateHeyaData {
    name: String,
}

#[post("/heya")]
pub async fn create(
    data: web::Form<CreateHeyaData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    let mut db = state.db.lock().unwrap();
    let player_id = identity.player_id()?;
    let heya = Heya::new(&mut db, &data.name, player_id)?;
    Ok(HttpResponse::SeeOther()
        .insert_header((http::header::LOCATION, heya.url_path()))
        .finish())
}
