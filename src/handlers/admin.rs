use super::{BaseTemplate, HandlerError, Result};
use crate::data::basho::backfill_past_player_ranks;
use crate::data::push::{
    mass_notify_basho_result, mass_notify_day_result, mass_notify_kyujyo, SendStats,
};
use crate::data::{self, basho, BashoId, DbConn, Player, PlayerId, Rank};
use crate::external::discord::DiscordAuthProvider;
use crate::external::google::GoogleAuthProvider;
use crate::external::reddit::RedditAuthProvider;
use crate::external::{sumo_api, AuthProvider};
use crate::AppState;

use actix_identity::Identity;
use actix_session::Session;
use actix_web::{get, http, post, web, HttpResponse, Responder};
use anyhow::anyhow;
use chrono::NaiveDateTime;
use futures::prelude::*;
use itertools::Itertools;
use regex::{Regex, RegexBuilder};
use rinja::Template;
use rusqlite::{Connection, OptionalExtension, Result as SqlResult};
use serde::{Deserialize, Deserializer};
use std::time::Duration;

#[derive(Template)]
#[template(path = "edit_basho.html")]
pub struct EditBashoTemplate {
    base: BaseTemplate,
    basho: BashoData,
}

#[get("/edit")]
pub async fn edit_basho_page(
    path: web::Path<BashoId>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<EditBashoTemplate> {
    let base = {
        let db = state.db.lock().unwrap();
        BaseTemplate::for_admin(&db, &identity, &state)?
    };
    Ok(EditBashoTemplate {
        base,
        basho: BashoData::with_id(&state.db, *path).await?,
    })
}

#[derive(Debug, Deserialize)]
pub struct BashoData {
    venue: String,
    #[serde(deserialize_with = "deserialize_datetime")]
    start_date: NaiveDateTime,
    banzuke: Vec<BanzukeRikishi>,
    notify_kyujyo: bool,
}

impl BashoData {
    async fn with_id(conn: &DbConn, id: BashoId) -> Result<Self> {
        let mut basho = {
            let db = conn.lock().unwrap();
            db.query_row(
                "
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
                        banzuke: Self::fetch_banzuke_from_db(&db, id)?,
                        notify_kyujyo: true,
                    })
                },
            )
            .optional()?
            .unwrap_or_else(|| Self::make_basho_stub(id))
        };

        if basho.banzuke.is_empty() {
            basho.banzuke = match Self::fetch_banzuke_from_sumo_api(id).await {
                Ok(val) => val,
                Err(e) => {
                    warn!("Failed to fetch {:#} banzuke from sumo-api: {}", id, e);
                    vec![]
                }
            }
        }

        Ok(basho)
    }

    fn make_basho_stub(id: BashoId) -> BashoData {
        Self {
            venue: id.expected_venue(),
            start_date: id.expected_start_date().naive_local(),
            banzuke: vec![],
            notify_kyujyo: false,
        }
    }

    fn fetch_banzuke_from_db(db: &Connection, id: BashoId) -> SqlResult<Vec<BanzukeRikishi>> {
        db.prepare(
            "
            SELECT family_name, rank, kyujyo
            FROM banzuke
            WHERE basho_id = ?",
        )?
        .query_map(params![id], |row| {
            //debug!("got banzuke row with name {:?}", row.get("family_name")?);
            Ok(BanzukeRikishi {
                name: row.get("family_name")?,
                rank: row.get("rank")?,
                is_kyujyo: row.get("kyujyo")?,
            })
        })?
        .collect::<SqlResult<Vec<BanzukeRikishi>>>()
    }

    async fn fetch_banzuke_from_sumo_api(id: BashoId) -> reqwest::Result<Vec<BanzukeRikishi>> {
        debug!("Fetching Makuuchi and Juryo banzuke from sumo-api");
        let makuuchi = sumo_api::BanzukeResponse::fetch(id, data::RankDivision::Makuuchi).await?;
        let juryo = sumo_api::BanzukeResponse::fetch(id, data::RankDivision::Juryo).await?;
        Ok(makuuchi
            .all_rikishi()
            .chain(juryo.all_rikishi())
            .map(|r| BanzukeRikishi {
                name: r.shikona_en.to_owned(),
                rank: r.rank,
                is_kyujyo: false,
            })
            .sorted_by_key(|r| r.rank)
            .collect())
    }
}

fn deserialize_datetime<'de, D>(deserializer: D) -> std::result::Result<NaiveDateTime, D::Error>
where
    D: Deserializer<'de>,
{
    let s: String = String::deserialize(deserializer)?;
    debug!("parsing datetime from {}", s);
    NaiveDateTime::parse_from_str(&s, "%FT%R")
        .or_else(|_e| NaiveDateTime::parse_from_str(&s, "%FT%T%.f"))
        .map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize)]
struct BanzukeRikishi {
    name: String,
    rank: Rank,
    is_kyujyo: bool,
}

#[derive(Debug, Serialize)]
pub struct BanzukeResponseData {
    basho_url: String,
    notification_stats: SendStats,
}

#[post("/edit")]
pub async fn edit_basho_post(
    path: web::Path<BashoId>,
    basho: web::Json<BashoData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<web::Json<BanzukeResponseData>> {
    let basho_id = *path;
    {
        let mut db = state.db.lock().unwrap();
        BaseTemplate::for_admin(&db, &identity, &state)?;
        data::basho::update_basho(
            &mut db,
            basho_id,
            &basho.venue,
            &basho.start_date,
            &basho
                .banzuke
                .iter()
                .map(|b| (b.name.to_owned(), b.rank.to_owned(), b.is_kyujyo))
                .collect::<Vec<_>>(),
        )?;
    }
    let notification_stats = if basho.notify_kyujyo {
        mass_notify_kyujyo(&state.db, &state.push, &state.config.url(), basho_id).await?
    } else {
        SendStats::default()
    };
    Ok(web::Json(BanzukeResponseData {
        basho_url: basho_id.url_path(),
        notification_stats,
    }))
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

#[get("/day/{day}")]
pub async fn torikumi_page(
    path: web::Path<(BashoId, u8)>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<TorikumiTemplate> {
    let basho_id = path.0;
    let day = path.1;
    let base = {
        let db = state.db.lock().unwrap();
        BaseTemplate::for_admin(&db, &identity, &state)?
    };
    let sumo_db_text = fetch_sumo_db_torikumi(basho_id, day)
        .map_ok(Some)
        .or_else(|e| async move {
            warn!("failed to fetch sumodb data: {}", e);
            Ok::<_, anyhow::Error>(None)
        })
        .await?;
    Ok(TorikumiTemplate {
        base,
        basho_id,
        day,
        sumo_db_text,
    })
}

async fn fetch_sumo_db_torikumi(basho_id: BashoId, day: u8) -> Result<String> {
    lazy_static! {
        static ref RE: Regex =
            RegexBuilder::new(r#"<div +class="simplecontent">\s*<pre>.*?\WMakuuchi\s+(.*?)</pre>"#)
                .dot_matches_new_line(true)
                .build()
                .unwrap();
    }

    let url = format!(
        "http://sumodb.sumogames.de/Results_text.aspx?b={}&d={}",
        basho_id.id(),
        day
    );
    debug!("sending request to {}", url);
    let str = reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent("kachiclash.com")
        .build()?
        .get(&url)
        .send()
        .await?
        .text()
        .await?;
    RE.captures(str.as_str())
        .map(|cap| cap.get(1).unwrap().as_str().to_string())
        .ok_or_else(|| anyhow!("sumodb response did not match regex").into())
}

#[derive(Debug, Deserialize)]
pub struct TorikumiData {
    torikumi: Option<Vec<data::basho::TorikumiMatchUpdateData>>,
    notify: bool,
}

#[post("/day/{day}")]
pub async fn torikumi_post(
    path: web::Path<(BashoId, u8)>,
    torikumi: web::Json<TorikumiData>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    BaseTemplate::for_admin(&state.db.lock().unwrap(), &identity, &state)?;
    let mut notify = torikumi.notify;
    if let Some(torikumi) = &torikumi.torikumi {
        let mut db = state.db.lock().unwrap();
        data::basho::update_torikumi(&mut db, path.0, path.1, torikumi)?;
    } else if !crate::poll::daily_results::query_and_update(path.0, path.1, &state, false).await? {
        notify = false;
    }

    let stats = if notify {
        mass_notify_day_result(&state.db, &state.push, &state.config.url(), path.0, path.1).await?
    } else {
        SendStats::default()
    };
    Ok(web::Json(stats))
}

#[post("/finalize")]
pub async fn finalize_basho(
    path: web::Path<BashoId>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    {
        let mut db = state.db.lock().unwrap();
        BaseTemplate::for_admin(&db, &identity, &state)?;
        basho::finalize_basho(&mut db, *path)?;
    }
    let stats =
        mass_notify_basho_result(&state.db, &state.push, &state.config.url(), *path).await?;
    Ok(web::Json(stats))
}

#[post("/backfill_player_ranks")]
pub async fn backfill_player_ranks(
    path: web::Path<BashoId>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    let mut db = state.db.lock().unwrap();
    BaseTemplate::for_admin(&db, &identity, &state)?;
    backfill_past_player_ranks(&mut db, *path)?;
    Ok(HttpResponse::SeeOther()
        .insert_header((http::header::LOCATION, &*path.url_path()))
        .finish())
}

#[derive(Template)]
#[template(path = "list_players.html")]
pub struct ListPlayersTemplate {
    base: BaseTemplate,
    players: Vec<Player>,
}

#[get("/player")]
pub async fn list_players(
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<ListPlayersTemplate> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::for_admin(&db, &identity, &state)?;
    Ok(ListPlayersTemplate {
        players: Player::list_all(&db, base.current_or_next_basho_id)?,
        base,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ImageUpdateData {
    service_name: String,
    player_ids: Vec<PlayerId>,
}

#[post("/player/update_images")]
pub async fn update_user_images(
    json: web::Json<ImageUpdateData>,
    state: web::Data<AppState>,
    session: Session,
) -> Result<HttpResponse> {
    let provider = match json.service_name.as_str() {
        "discord" => Ok(Box::new(DiscordAuthProvider) as Box<dyn AuthProvider>),
        "google" => Ok(Box::new(GoogleAuthProvider) as Box<dyn AuthProvider>),
        "reddit" => Ok(Box::new(RedditAuthProvider) as Box<dyn AuthProvider>),
        _ => Err(HandlerError::NotFound("auth service".to_string())),
    }?;
    session
        .insert("image_update_data", &json.0)
        .map_err(anyhow::Error::from)?;
    let (auth_url, csrf_token) = provider.authorize_url(&state.config);
    session
        .insert("oauth_csrf", csrf_token)
        .map_err(anyhow::Error::from)?;
    Ok(HttpResponse::SeeOther()
        .insert_header((http::header::LOCATION, auth_url.to_string()))
        .finish())
}
