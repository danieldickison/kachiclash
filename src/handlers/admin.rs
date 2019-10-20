
use crate::data::{self, Rank, BashoId, DbConn};
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
use futures::future::{self};
use crate::handlers::admin::SumoDbError::HttpRequestError;

#[derive(Template)]
#[template(path = "edit_basho.html")]
pub struct EditBashoTemplate {
    base: BaseTemplate,
    basho: Option<BashoData>,
}

pub fn new_basho_page(state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<EditBashoTemplate>> {
    Ok(EditBashoTemplate {
        base: admin_base(&state.db, &identity)?,
        basho: None,
    }.into())
}

pub fn edit_basho_page(path: web::Path<BashoId>, state: web::Data<AppState>, identity: Identity) -> Result<AskamaResponder<EditBashoTemplate>> {
    match BashoData::with_id(&state.db.lock().unwrap(), *path)? {
        Some(basho) =>
            Ok(EditBashoTemplate {
                base: admin_base(&state.db, &identity)?,
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
                //debug!("got basho row start date {:?}", row.get("start_date")?);
                Ok(Self {
                    start_date: row.get("start_date")?,
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
    admin_base(&state.db, &identity)?;
    let basho_id = data::basho::make_basho(
        &mut state.db.lock().unwrap(),
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

struct AdminBaseFuture {
    db: DbConn,
    identity: Identity,
}

impl Future for AdminBaseFuture {
    type Item = BaseTemplate;
    type Error = failure::Error;

    fn poll(&mut self) -> Poll<Self::Item, Self::Error> {
        let base = BaseTemplate::new(&self.db.lock().unwrap(), &self.identity)?;
        if base.player.as_ref().map_or(false, |p| p.is_admin()) {
            Ok(Async::Ready(base))
        } else {
            Err(HandlerError::MustBeLoggedIn.into())
        }
    }
}

fn admin_base(db: &DbConn, identity: &Identity) -> Result<BaseTemplate> {
    AdminBaseFuture {
        db: db.clone(),
        identity: identity.clone(),
    }.wait()
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
    AdminBaseFuture {
        db: state.db.clone(),
        identity: identity.clone(),
    }.join(fetch_sumo_db_torikumi(basho_id, day))
        .then(move |result| {
            // TODO: make base error fatal but sumo_db_text error optional
            result.map(|(base, sumo_db_text)| {
                TorikumiTemplate {
                    base: base,
                    basho_id: basho_id,
                    day: day,
                    sumo_db_text: Some(sumo_db_text),
                }.into()
            })
        })
}

fn fetch_sumo_db_torikumi(basho_id: BashoId, day: u8)
    -> impl Future<Item = String, Error = failure::Error> {

    let mut client = Client::default();
    let url = format!("http://sumodb.sumogames.de/Results_text.aspx?b={}&d={}", basho_id.id(), day);
    client.get(url)
        .header("User-Agent", "kachiclash")
        .send()
        .map_err(|_| SumoDbError::HttpRequestError.into())
        .and_then(|mut response| {
            response.body().map_err(|_| SumoDbError::ParseError.into())
        })
        .and_then(|body| {
            String::from_utf8(body.to_vec()).map_err(|e| {
                warn!("failed to parse sumodb response as utf8: {}", e);
                SumoDbError::ParseError.into()
            })
        })
}

#[derive(Fail, Debug)]
enum SumoDbError {
    #[fail(display = "SumoDB http request failed")]
    HttpRequestError,

    #[fail(display = "SumoDB response could not be parsed")]
    ParseError,
}

#[derive(Debug, Deserialize)]
pub struct TorikumiData {
    torikumi: Vec<data::basho::TorikumiMatchUpdateData>,
}

pub fn torikumi_post(path: web::Path<(BashoId, u8)>, torikumi: web::Json<TorikumiData>, state: web::Data<AppState>, identity: Identity)
-> Result<()> {
    admin_base(&state.db, &identity)?;
    data::basho::update_torikumi(
        &mut state.db.lock().unwrap(),
        path.0,
        path.1,
        &torikumi.torikumi
    ).map_err(|e| e.into())
}
