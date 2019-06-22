extern crate askama;

// use super::data::{
//     ActivitiesResponse, ActivityRequest, ActivityResponse, EditActivityRequest, ErrorListResponse,
// };
use super::data;
// use super::external;
use super::AppState;
use actix_web::{error, Error, HttpRequest, HttpResponse, Json, Path, Responder};
use actix_web::middleware::session::RequestSession;
use failure::Fail;
use askama::Template;

#[derive(Fail, Debug)]
pub enum KachiClashError {
    #[fail(display = "External Service Error")]
    ExternalServiceError,
    #[fail(display = "Database Error")]
    DatabaseError,
}

impl error::ResponseError for KachiClashError {
    fn error_response(&self) -> HttpResponse {
        match *self {
            KachiClashError::ExternalServiceError => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body("external service error"),
            KachiClashError::DatabaseError => HttpResponse::InternalServerError()
                .content_type("text/plain")
                .body("database error"),
        }
    }
}

#[derive(Template)]
#[template(path = "index.html")]
struct IndexTemplate {
    leaders: Vec<data::Player>,
}

pub fn index(req: &HttpRequest<AppState>) -> impl Responder {
    let log = &req.state().log;
    if let Some(count) = req.session().get::<i32>("counter").unwrap_or(None) {
        debug!(log, "SESSION counter: {}", count);
        if let Err(e) = req.session().set("counter", count+1) {
            warn!(log, "could not increment counter: {:?}", e);
        }
    } else {
        debug!(log, "SESSION init counter to 0");
        if let Err(e) = req.session().set("counter", 1) {
            warn!(log, "could not initialize counter: {:?}", e);
        }
    }

    IndexTemplate {
        leaders: data::list_players(&req.state().db)
    }
}

pub fn list_players(req: &HttpRequest<AppState>) -> impl Responder {
    data::list_players(&req.state().db)
        .iter()
        .map(|p| {
            format!("{}: {} joined {}", p.id, p.name, p.join_date)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn name(req: &HttpRequest<AppState>) -> impl Responder {
    let log = &req.state().log;
    data::get_name(&req.state().db)
        .map_err(|err| {
            error!(log, "db error: {}", err);
            KachiClashError::DatabaseError
        })
}

pub fn health(req: &HttpRequest<AppState>) -> impl Responder {
    let log = &req.state().log;
    debug!(log, "health hit");
    "OK".to_string()
}

pub fn json_error_handler(err: error::JsonPayloadError, _: &HttpRequest<AppState>) -> Error {
    error::InternalError::from_response(
        "",
        HttpResponse::BadRequest()
            .content_type("application/json")
            .body(format!(r#"{{"error":"json error: {}"}}"#, err)),
    )
    .into()
}
