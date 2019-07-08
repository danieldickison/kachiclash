extern crate askama;

use super::data;
use super::AppState;
use actix_web::{error, HttpResponse, Responder};
use actix_web::web::Data;
use actix_identity::Identity;
use failure::Fail;
use askama::Template;

pub mod basho;
pub mod login;

#[derive(Fail, Debug)]
pub enum KachiClashError {
    #[fail(display = "External Service Error")]
    ExternalServiceError,
    
    #[fail(display = "Database Error")]
    DatabaseError,

    #[fail(display = "CSRF Error")]
    CSRFError,
}

impl error::ResponseError for KachiClashError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            KachiClashError::ExternalServiceError => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body(format!("{}", self)),
            KachiClashError::DatabaseError => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body(format!("{}", self)),
            KachiClashError::CSRFError => HttpResponse::Forbidden()
                .content_type("text/plain")
                .body(format!("{}", self)),
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    leaders: Vec<data::player::Player>,
}

pub fn index(state: Data<AppState>, identity: Identity) -> impl Responder {
    debug!("Identity: {:?}", identity.identity());

    let s = IndexTemplate {
        leaders: data::player::list_players(&state.db)
    }.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

pub fn list_players(state: Data<AppState>) -> impl Responder {
    data::player::list_players(&state.db)
        .iter()
        .map(|p| {
            format!("{}: {} joined {}", p.id, p.name, p.join_date)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

// pub fn json_error_handler(err: error::JsonPayloadError, _: &HttpRequest<AppState>) -> Error {
//     error::InternalError::from_response(
//         "",
//         HttpResponse::BadRequest()
//             .content_type("application/json")
//             .body(format!(r#"{{"error":"json error: {}"}}"#, err)),
//     )
//     .into()
// }
