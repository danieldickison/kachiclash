extern crate askama;

use crate::data::{Player, DataError};

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
pub mod settings;

type Result<T> = std::result::Result<T, failure::Error>;

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        match self {
            HandlerError::NotFound(_) => HttpResponse::NotFound(),
            HandlerError::ExternalServiceError => HttpResponse::InternalServerError(),
            HandlerError::DatabaseError(_) => HttpResponse::InternalServerError(),
            HandlerError::CSRFError | HandlerError::MustBeLoggedIn => HttpResponse::Forbidden(),
        }
            .content_type("text/plain")
            .body(self.to_string())
    }
}

#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "{} not found", _0)]
    NotFound(String),

    #[fail(display = "Must be logged in")]
    MustBeLoggedIn,

    #[fail(display = "External service error")]
    ExternalServiceError,

    #[fail(display = "Database error: {}", _0)]
    DatabaseError(DataError),

    #[fail(display = "CSRF error")]
    CSRFError,
}

struct BaseTemplate {
    player: Option<Player>,
}

impl BaseTemplate {
    fn new(db: &Connection, identity: &Identity) -> Result<Self> {
        let player = match identity.identity() {
            Some(id) => {
                let player = Player::with_id(&db, id.parse()?)?;
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
