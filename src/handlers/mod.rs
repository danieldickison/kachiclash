extern crate askama;

use crate::data;

use actix_web::{error, HttpResponse};
use actix_identity::Identity;
use rusqlite::Connection;
use failure::Fail;

mod askama_responder;
use askama_responder::AskamaResponder;

pub mod index;
pub mod basho;
pub mod login;
pub mod admin;

type Result<T> = std::result::Result<T, failure::Error>;

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

    fn is_admin(&self) -> bool {
        match &self.player {
            Some(p) => p.is_admin(),
            None => false
        }
    }
}
