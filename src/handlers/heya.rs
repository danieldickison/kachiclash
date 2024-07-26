use actix_identity::Identity;
use actix_web::{get, http, post, web, HttpResponse, Responder};
use askama::Template;

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
    match Heya::with_slug(&db, &path)? {
        Some(heya) => Ok(HeyaTemplate {
            is_oyakata: player_id.map_or(false, |pid| pid == heya.oyakata_player_id),
            base,
            heya,
        }),
        None => Err(HandlerError::NotFound("heya".to_string())),
    }
}

#[derive(Debug, Deserialize)]
pub struct AddMemberData {
    player_id: PlayerId,
}

#[post("/member")]
pub async fn add_member(
    path: web::Path<String>,
    data: web::Json<AddMemberData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    match Heya::with_slug(&db, &path)? {
        Some(heya) => {
            if heya.oyakata_player_id == identity.player_id()? {
                heya.add_member(&db, data.0.player_id)?;
                Ok(HttpResponse::Ok())
            } else {
                Err(HandlerError::MustBeLoggedIn)
            }
        }
        None => Err(HandlerError::NotFound("heya".to_string())),
    }
}

#[derive(Template)]
#[template(path = "heya_list.html")]
pub struct HeyaListTemplate {
    base: BaseTemplate,
    heyas: Vec<Heya>,
}

#[get("/heya")]
pub async fn list(
    state: web::Data<AppState>,
    identity: Option<Identity>,
) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    Ok(HeyaListTemplate {
        base: BaseTemplate::new(&db, identity.as_ref(), &state)?,
        heyas: Heya::list_all(&db)?,
    })
}

#[derive(Debug, Deserialize)]
pub struct CreateHeyaData {
    name: String,
}

#[post("/heya")]
pub async fn create(
    data: web::Json<CreateHeyaData>,
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
