extern crate askama;

use crate::data::{Player, DataError, PlayerId};

use actix_web::{error, HttpResponse};
use actix_identity::Identity;
use rusqlite::Connection;
use failure::Fail;

pub mod index;
pub mod basho;
pub mod login;
pub mod admin;
pub mod settings;

type Result<T> = std::result::Result<T, HandlerError>;

#[derive(Fail, Debug)]
pub enum HandlerError {
    #[fail(display = "{} not found", _0)]
    NotFound(String),

    #[fail(display = "Must be logged in")]
    MustBeLoggedIn,

    #[fail(display = "External service error")]
    ExternalServiceError,

    #[fail(display = "Database error")]
    DatabaseError(DataError),

    #[fail(display = "CSRF error")]
    CSRFError,

    #[fail(display = "Unexpected failure")]
    Failure(failure::Error),
}

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        debug!("HandlerError {:?}, responding with error message: {}", self, self);
        match self {
            HandlerError::NotFound(_) => HttpResponse::NotFound(),
            HandlerError::ExternalServiceError => HttpResponse::InternalServerError(),
            HandlerError::DatabaseError(_) | HandlerError::Failure(_) => HttpResponse::InternalServerError(),
            HandlerError::CSRFError | HandlerError::MustBeLoggedIn => HttpResponse::Forbidden(),
        }
            .content_type("text/plain")
            .body(self.to_string())
    }
}

impl From<DataError> for HandlerError {
    fn from(err: DataError) -> Self {
        Self::DatabaseError(err)
    }
}

impl From<failure::Error> for HandlerError {
    fn from(err: failure::Error) -> Self {
        Self::Failure(err)
    }
}

struct BaseTemplate {
    player: Option<Player>,
}

impl BaseTemplate {
    fn new(db: &Connection, identity: &Identity) -> Result<Self> {
        let player = match identity.player_id() {
            Some(id) => {
                let player = Player::with_id(&db, id)?;
                match player.as_ref() {
                    Some(p) => debug!("Logged in player: {} ({})", p.name, p.id),
                    None => {
                        error!("identity player id {} not found; forcing log out", id);
                        identity.forget();
                    }
                };
                player
            },
            None => None,
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

trait IdentityExt {
    fn player_id(&self) -> Option<PlayerId>;

    fn require_player_id(&self) -> Result<PlayerId> {
        self.player_id().ok_or(HandlerError::MustBeLoggedIn)
    }
}

impl IdentityExt for Identity {
    fn player_id(&self) -> Option<PlayerId> {
        self.identity()
            .and_then(|str| str.parse().ok())
    }
}
