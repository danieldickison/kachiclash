extern crate askama;

use crate::data::{Player, DataError, PlayerId};

use actix_web::{error, HttpResponse};
use actix_identity::Identity;
use rusqlite::Connection;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub mod index;
pub mod basho;
pub mod login;
pub mod admin;
pub mod settings;
pub mod player;

type Result<T> = std::result::Result<T, HandlerError>;

#[derive(Debug)]
pub enum HandlerError {
    NotFound(String),
    MustBeLoggedIn,
    ExternalServiceError,
    DatabaseError(DataError),
    CSRFError,
    Failure(anyhow::Error),
    ActixError(String),
}

impl Error for HandlerError {}

impl Display for HandlerError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            HandlerError::NotFound(thing) => write!(f, "{} not found", thing),
            HandlerError::MustBeLoggedIn => write!(f, "Must be logged in"),
            HandlerError::ExternalServiceError => write!(f, "External service error"),
            HandlerError::DatabaseError(_) => write!(f, "Database error"),
            HandlerError::CSRFError => write!(f, "CRSF error"),
            HandlerError::Failure(_) => write!(f, "Unexpected failure"),
            HandlerError::ActixError(_) => write!(f, "actix-web error"),
        }?;
        Ok(())
    }
}

impl error::ResponseError for HandlerError {
    fn error_response(&self) -> HttpResponse {
        debug!("HandlerError {:?}, responding with error message: {}", self, self);
        match self {
            HandlerError::NotFound(_) => HttpResponse::NotFound(),
            HandlerError::ExternalServiceError => HttpResponse::InternalServerError(),
            HandlerError::DatabaseError(_) | HandlerError::Failure(_) | HandlerError::ActixError(_) => HttpResponse::InternalServerError(),
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

impl From<anyhow::Error> for HandlerError {
    fn from(err: anyhow::Error) -> Self {
        Self::Failure(err)
    }
}

impl From<reqwest::Error> for HandlerError {
    fn from(_err: reqwest::Error) -> Self {
        Self::ExternalServiceError
    }
}

impl From<actix_web::Error> for HandlerError {
    fn from(err: actix_web::Error) -> Self {
        // I can't figure out how to make the actix error Send+Sync so just make it a string for now.
        Self::ActixError(err.to_string())
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
