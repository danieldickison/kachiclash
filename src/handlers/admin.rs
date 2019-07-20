
use crate::data::{self, Rank, BashoId};
use crate::AppState;
use super::{HandlerError, BaseTemplate, Result, AskamaResponder};

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
        base,
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

#[derive(Debug, Serialize)]
pub struct BanzukeResponseData {
    basho_url: String,
}

pub fn new_basho_post(basho: web::Json<BashoData>, state: web::Data<AppState>, identity: Identity)
-> Result<web::Json<BanzukeResponseData>> {
    let mut db = state.db.lock().unwrap();
    admin_base(&db, &identity)?;
    let basho_id = data::basho::make_basho(
        &mut db,
        &basho.venue,
        &basho.start_date,
        &basho.banzuke
            .iter()
            .map(|b| (b.name.to_owned(), b.rank.to_owned()))
            .collect::<Vec<_>>()
    )?;
    Ok(web::Json(BanzukeResponseData {
        basho_url: basho_id.url_path()
    }))
}

fn admin_base(db: &Connection, identity: &Identity) -> Result<BaseTemplate> {
    let base = BaseTemplate::new(&db, &identity)?;

    if base.player.as_ref().map_or(false, |p| p.is_admin()) {
        Ok(base)
    } else {
        Err(HandlerError::MustBeLoggedIn.into())
    }
}


///////

#[derive(Template)]
#[template(path = "torikumi.html")]
pub struct TorikumiTemplate {
    base: BaseTemplate,
}

pub fn torikumi_page(state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<TorikumiTemplate>> {
    let db = state.db.lock().unwrap();
    Ok(TorikumiTemplate {
        base: admin_base(&db, &identity)?,
    }.into())
}

#[derive(Debug, Deserialize)]
pub struct TorikumiData {
    torikumi: Vec<data::basho::TorikumiMatchUpdateData>,
}

pub fn torikumi_post(path: web::Path<(BashoId, u8)>, torikumi: web::Json<TorikumiData>, state: web::Data<AppState>, identity: Identity)
-> Result<()> {
    let mut db = state.db.lock().unwrap();
    admin_base(&db, &identity)?;
    data::basho::update_torikumi(
        &mut db,
        &path.0,
        &path.1,
        &torikumi.torikumi
    ).map_err(|e| e.into())
}
