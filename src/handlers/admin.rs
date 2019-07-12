use crate::data;
use crate::AppState;
use super::{HandlerError, BaseTemplate, Result};

use actix_web::{HttpResponse, Responder};
use actix_web::web::Data;
use actix_identity::Identity;
use rusqlite::Connection;
use askama::Template;


#[derive(Template)]
#[template(path = "admin.html")]
struct AdminIndexTemplate {
    base: BaseTemplate,
}

pub fn index(state: Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity)?;

    if !base.player.as_ref().map_or(false, |p| p.is_admin()) {
        return Err(HandlerError::MustBeLoggedIn.into());
    }

    let s = AdminIndexTemplate {
        base: base,
    }.render().unwrap();
    Ok(HttpResponse::Ok().body(s))
}
