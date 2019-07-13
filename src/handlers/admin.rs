
use crate::data::{Rank};
use crate::AppState;
use super::{HandlerError, BaseTemplate, Result};

use actix_web::{HttpResponse, Responder};
use actix_web::web;
use actix_identity::Identity;
use rusqlite::Connection;
use askama::Template;
use serde::{Deserializer, Deserialize};
use chrono::NaiveDateTime;

#[derive(Template)]
#[template(path = "new_basho.html")]
struct NewBashoTemplate {
    base: BaseTemplate,
}

pub fn new_basho_page(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    let db = state.db.lock().unwrap();
    let base = admin_base(&db, &identity)?;

    let s = NewBashoTemplate {
        base: base,
    }.render().unwrap();
    Ok(HttpResponse::Ok().body(s))
}

#[derive(Debug, Deserialize)]
pub struct BashoData {
    venue: String,
    #[serde(deserialize_with="deserialize_datetime")]
    start_date: NaiveDateTime,
    banzuke: Vec<BanzukeRikishi>,
}

fn deserialize_datetime<'de, D>(deserializer: D) -> std::result::Result<NaiveDateTime, D::Error>
        where D: Deserializer<'de> {
    let s: String = String::deserialize(deserializer)?;
    debug!("parsing datetime from {}", s);
    NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M").map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct BanzukeRikishi {
    name: String,
    rank: Rank,
}

pub fn new_basho_post(basho: web::Json<BashoData>, state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    admin_base(&state.db.lock().unwrap(), &identity)?;
    debug!("{:?}", basho.venue);
    Ok("{}")
}

fn admin_base(db: &Connection, identity: &Identity) -> Result<BaseTemplate> {
    let base = BaseTemplate::new(&db, &identity)?;

    if base.player.as_ref().map_or(false, |p| p.is_admin()) {
        Ok(base)
    } else {
        Err(HandlerError::MustBeLoggedIn.into())
    }
}
