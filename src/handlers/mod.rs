extern crate askama;

use crate::data::{BashoId, BashoInfo, DataError, Player, PlayerId};
use crate::AppState;

use actix_identity::Identity;
use actix_web::{error, web, HttpResponse};
use rusqlite::Connection;
use std::error::Error;
use std::fmt::{Display, Formatter};

pub mod admin;
pub mod basho;
pub mod index;
pub mod login;
pub mod player;
pub mod push;
pub mod settings;
pub mod stats;

mod user_agent;

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
        debug!(
            "HandlerError {:?}, responding with error message: {}",
            self, self
        );
        match self {
            HandlerError::NotFound(_) => HttpResponse::NotFound(),
            HandlerError::ExternalServiceError
            | HandlerError::DatabaseError(_)
            | HandlerError::Failure(_)
            | HandlerError::ActixError(_) => HttpResponse::InternalServerError(),
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

impl From<rusqlite::Error> for HandlerError {
    fn from(err: rusqlite::Error) -> Self {
        Self::DatabaseError(DataError::from(err))
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
    current_or_next_basho_id: BashoId,
    vapid_public_key: String,
}

impl BaseTemplate {
    fn new(
        db: &Connection,
        identity: Option<&Identity>,
        state: &web::Data<AppState>,
    ) -> Result<Self> {
        let current_or_next_basho_id = BashoInfo::current_or_next_basho_id(&db)?;
        let player = match identity {
            None => None,
            Some(id) => {
                let player_id = id.player_id()?;
                Some(
                    Player::with_id(db, player_id, current_or_next_basho_id)?.ok_or_else(|| {
                        error!("identity player id {} not found", player_id);
                        HandlerError::NotFound("player".to_string())
                    })?,
                )
            }
        };
        let vapid_public_key = state.config.vapid_public_key.clone();
        Ok(Self {
            player,
            current_or_next_basho_id,
            vapid_public_key,
        })
    }

    fn is_admin(&self) -> bool {
        match &self.player {
            Some(p) => p.is_admin(),
            None => false,
        }
    }
}

trait IdentityExt {
    fn player_id(&self) -> anyhow::Result<PlayerId>;
}

impl IdentityExt for Identity {
    fn player_id(&self) -> anyhow::Result<PlayerId> {
        Ok(self.id()?.parse()?)
    }
}
