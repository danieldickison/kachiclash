extern crate askama;

// use super::data::{
//     ActivitiesResponse, ActivityRequest, ActivityResponse, EditActivityRequest, ErrorListResponse,
// };
use super::data;
// use super::external;
use super::AppState;
use actix_web::{error, HttpRequest, HttpResponse, Responder};
use actix_web::web::Data;
use actix_session::Session;
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

pub fn index(state: Data<AppState>, session: Session) -> impl Responder {
    if let Some(count) = session.get::<i32>("counter").unwrap_or(None) {
        debug!("SESSION counter: {}", count);
        if let Err(e) = session.set("counter", count+1) {
            warn!("could not increment counter: {:?}", e);
        }
    } else {
        debug!("SESSION init counter to 0");
        if let Err(e) = session.set("counter", 1) {
            warn!("could not initialize counter: {:?}", e);
        }
    }

    let s = IndexTemplate {
        leaders: data::list_players(&state.db)
    }.render().unwrap();
    HttpResponse::Ok().content_type("text/html").body(s)
}

pub fn list_players(state: Data<AppState>) -> impl Responder {
    data::list_players(&state.db)
        .iter()
        .map(|p| {
            format!("{}: {} joined {}", p.id, p.name, p.join_date)
        })
        .collect::<Vec<String>>()
        .join("\n")
}

pub fn name(state: Data<AppState>) -> impl Responder {
    data::get_name(&state.db)
        .map_err(|err| {
            error!("db error: {}", err);
            KachiClashError::DatabaseError
        })
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
