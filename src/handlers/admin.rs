
use crate::data::{self, Rank, BashoId};
use crate::AppState;
use super::{HandlerError, BaseTemplate, Result, AskamaResponder};

use actix_web::web;
use actix_identity::Identity;
use rusqlite::{Connection, Result as SqlResult};
use askama::Template;
use serde::{Deserializer, Deserialize};
use chrono::NaiveDateTime;
use result::prelude::*;

#[derive(Template)]
#[template(path = "edit_basho.html")]
pub struct EditBashoTemplate {
    base: BaseTemplate,
    basho: Option<BashoData>,
}

pub fn edit_basho_page(path: web::Path<Option<BashoId>>, state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<EditBashoTemplate>> {
    let db = state.db.lock().unwrap();
    Ok(EditBashoTemplate {
        base: admin_base(&db, &identity)?,
        basho: path.map(|id| BashoData::with_id(&db, id)).invert()?,
    }.into())
}

#[derive(Debug, Deserialize)]
pub struct BashoData {
    venue: String,
    #[serde(deserialize_with="deserialize_datetime")]
    start_date: NaiveDateTime,
    banzuke: Vec<BanzukeRikishi>,
}

impl BashoData {
    fn with_id(db: &Connection, id: BashoId) -> Result<Self> {
        db.query_row("
            SELECT
                basho.start_date,
                basho.venue
            FROM basho
            WHERE basho.id = ?",
            params![id],
            |row| {
                Ok(Self {
                    start_date: row.get("start_date")?,
                    venue: row.get("venue")?,
                    banzuke: Self::fetch_banzuke(&db, id)?,
                 })
            })
            .map_err(|e| e.into())
    }

    fn fetch_banzuke(db: &Connection, id: BashoId) -> SqlResult<Vec<BanzukeRikishi>> {
        db.prepare("
            SELECT family_name, rank
            FROM banzuke
            WHERE basho_id = ?")?
            .query_map(
                params![id],
                |row| {
                    Ok(BanzukeRikishi {
                        name: row.get("family_name")?,
                        rank: row.get("rank")?,
                    })
                })?
            .collect::<SqlResult<Vec<BanzukeRikishi>>>()
    }
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

pub fn edit_basho_post(basho: web::Json<BashoData>, state: web::Data<AppState>, identity: Identity)
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
    basho_id: BashoId,
    day: u8,
}

pub fn torikumi_page(path: web::Path<(BashoId, u8)>, state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<TorikumiTemplate>> {
    let db = state.db.lock().unwrap();
    Ok(TorikumiTemplate {
        base: admin_base(&db, &identity)?,
        basho_id: path.0,
        day: path.1,
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
        path.0,
        path.1,
        &torikumi.torikumi
    ).map_err(|e| e.into())
}
