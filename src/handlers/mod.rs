extern crate askama;

use crate::data;
use crate::AppState;

use actix_web::{error, HttpResponse, Responder};
use actix_web::web::Data;
use actix_identity::Identity;
use rusqlite::Connection;
use failure::Fail;
use askama::Template;

mod askama_responder;
use askama_responder::AskamaResponder;

pub mod basho;
pub mod login;
pub mod admin;

type Result<T> = std::result::Result<T, failure::Error>;

#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "Not Found")]
    NotFound(String),

    #[fail(display = "Must be logged in")]
    MustBeLoggedIn,

    #[fail(display = "External Service Error")]
    ExternalServiceError,
    
    #[fail(display = "Database Error")]
    DatabaseError,

    #[fail(display = "CSRF Error")]
    CSRFError,
}

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            HandlerError::NotFound(thing) => HttpResponse::NotFound()
                .content_type("text/plain")
                .body(format!("{} not found", thing)),
            HandlerError::ExternalServiceError => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body(format!("{}", self)),
            HandlerError::DatabaseError => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body(format!("{}", self)),
            HandlerError::CSRFError => HttpResponse::Forbidden()
                .content_type("text/plain")
                .body(format!("{}", self)),
            HandlerError::MustBeLoggedIn => HttpResponse::Forbidden()
                .content_type("text/plain")
                .body(format!("{}", self)),
        }
    }
}

struct BaseTemplate {
    player: Option<data::player::Player>,
}

impl BaseTemplate {
    fn new(db: &Connection, identity: &Identity) -> Result<Self> {
        let player = match identity.identity() {
            Some(id) => {
                let player = data::player::player_info(&db, id.parse()?)?;
                if player.is_none() {
                    error!("identity player id {} not found; forcing log out", id);
                    identity.forget();
                }
                player
            },
            None => None
        };
        Ok(Self {
            player
        })
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    base: BaseTemplate,
    leaders: Vec<data::player::Player>,
}

pub fn index(state: Data<AppState>, identity: Identity) -> Result<impl Responder> {
    debug!("Identity: {:?}", identity.identity());

    let db = state.db.lock().unwrap();

    let s = IndexTemplate {
        base: BaseTemplate::new(&db, &identity)?,
        leaders: data::player::list_players(&db)
    }.render().unwrap();
    Ok(HttpResponse::Ok().body(s))
}

pub fn list_players(state: Data<AppState>) -> impl Responder {
    data::player::list_players(&state.db.lock().unwrap())
        .iter()
        .map(|p| {
            format!("{}: {} joined {}", p.id, p.name, p.join_date)
        })
        .collect::<Vec<String>>()
        .join("\n")
}
