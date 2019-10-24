
use crate::data::{self, Rank, BashoId};
use crate::AppState;
use super::{HandlerError, BaseTemplate, Result, AskamaResponder};

use actix_web::web;
use actix_identity::Identity;
use actix_web::client::Client;
use rusqlite::{Connection, Result as SqlResult, OptionalExtension};
use askama::Template;
use serde::{Deserializer, Deserialize};
use chrono::NaiveDateTime;
use futures::prelude::*;
use regex::{Regex, RegexBuilder};

#[derive(Template)]
#[template(path = "edit_basho.html")]
pub struct EditBashoTemplate {
    base: BaseTemplate,
    basho: Option<BashoData>,
}

pub fn new_basho_page(state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<EditBashoTemplate>> {
    Ok(EditBashoTemplate {
        base: admin_base(&state.db.lock().unwrap(), &identity)?,
        basho: None,
    }.into())
}

pub fn edit_basho_page(path: web::Path<BashoId>, state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<EditBashoTemplate>> {
    let db = state.db.lock().unwrap();
    match BashoData::with_id(&db, *path)? {
        Some(basho) =>
            Ok(EditBashoTemplate {
                base: admin_base(&db, &identity)?,
                basho: Some(basho),
            }.into()),
        None => Err(HandlerError::NotFound("basho".to_string()).into())
    }
}

#[derive(Debug, Deserialize)]
pub struct BashoData {
    venue: String,
    #[serde(deserialize_with="deserialize_datetime")]
    start_date: NaiveDateTime,
    banzuke: Vec<BanzukeRikishi>,
}

impl BashoData {
    fn with_id(db: &Connection, id: BashoId) -> Result<Option<Self>> {
        db.query_row("
            SELECT
                basho.start_date,
                basho.venue
            FROM basho
            WHERE basho.id = ?",
            params![id],
            |row| {
                let start_date: NaiveDateTime = row.get("start_date")?;
                debug!("got basho row start date {:?}", start_date);
                Ok(Self {
                    start_date,
                    venue: row.get("venue")?,
                    banzuke: Self::fetch_banzuke(&db, id)?,
                 })
            })
            .optional()
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
                    //debug!("got banzuke row with name {:?}", row.get("family_name")?);
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
    sumo_db_text: Option<String>,
}

pub fn torikumi_page(path: web::Path<(BashoId, u8)>, state: web::Data<AppState>, identity: Identity)
    -> impl Future<Item = AskamaResponder<TorikumiTemplate>, Error = failure::Error> {

    let basho_id = path.0;
    let day = path.1;
    let db = state.db.lock().unwrap();
    admin_base(&db, &identity).into_future()
        .and_then(move |base| {
            fetch_sumo_db_torikumi(basho_id, day)
                .map(|txt| Some(txt))
                .or_else(|e| {
                    warn!("failed to fetch sumodb data: {}", e);
                    Ok(None)
                })
                .map(|opt_txt| (base, opt_txt))
        })
        .map(move |(base, sumo_db_text)| {
            TorikumiTemplate {
                base: base,
                basho_id: basho_id,
                day: day,
                sumo_db_text: sumo_db_text,
            }.into()
        })
}

fn fetch_sumo_db_torikumi(basho_id: BashoId, day: u8)
    -> impl Future<Item = String, Error = failure::Error> {

    lazy_static! {
        static ref RE: Regex =
            RegexBuilder::new(r#"<div +class="simplecontent">\s*<pre>.*?\WMakuuchi\s+(.*?)</pre>"#)
                .dot_matches_new_line(true)
                .build()
                .unwrap();
    }

    let client = Client::default();
    let url = format!("http://sumodb.sumogames.de/Results_text.aspx?b={}&d={}", basho_id.id(), day);
    client.get(url)
        .header("User-Agent", "kachiclash")
        .send()
        .map_err(|e| format_err!("{}", e))
        .and_then(|mut response| {
            response.body().map_err(|e| format_err!("{}", e))
        })
        .and_then(|body| {
            String::from_utf8(body.to_vec())
                .map_err(|e| format_err!("{}", e))
                .and_then(|str| {
                    RE.captures(str.as_str())
                        .map(|cap| cap.get(1).unwrap().as_str().to_string())
                        .ok_or_else(|| format_err!("sumodb response did not match regex"))
                })
        })
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
