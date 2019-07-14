use futures::future::err;
use futures::future::ok;
use futures::future::FutureResult;
use askama::Template;
use actix_web::{HttpRequest, HttpResponse, Responder};
use actix_web::http::StatusCode;
use failure::Error;

pub struct AskamaResponder<T: Template> (T);

impl<T: Template> From<T> for AskamaResponder<T> {
    fn from(template: T) -> Self {
        Self(template)
    }
}

impl<T: Template> Responder for AskamaResponder<T> {
    type Error = Error;
    type Future = FutureResult<HttpResponse, Error>;

    fn respond_to(self, _: &HttpRequest) -> Self::Future {

        match self.0.render() {
            Ok(str) => {
                ok(HttpResponse::build(StatusCode::OK)
                    .content_type("text/html; charset=utf-8")
                    .body(str))
            }
            Err(e) => err(e.into()),
        }
    }
}
