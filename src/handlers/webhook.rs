use std::ops::Deref;

use actix_identity::Identity;
use actix_web::http::header::{self, from_one_raw_str, ContentType, TryIntoHeaderValue};
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use anyhow::anyhow;
use url::Url;

use super::{BaseTemplate, Result};
use crate::external::sumo_api;
use crate::AppState;

#[post("/register")]
pub async fn register(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    BaseTemplate::for_admin(&state.db.lock().unwrap(), &identity, &state)?;
    sumo_api::register_webhook(&state.config).await?;
    Ok("Successfully registered")
}

#[derive(Deserialize)]
struct TestParams {
    #[serde(rename = "type")]
    webhook_type: String,
}

#[post("/test")]
pub async fn request_test(
    state: web::Data<AppState>,
    query: web::Form<TestParams>,
    identity: Identity,
) -> Result<impl Responder> {
    BaseTemplate::for_admin(&state.db.lock().unwrap(), &identity, &state)?;
    sumo_api::request_webhook_test(&state.config, &query.webhook_type).await?;
    Ok("Test request sent")
}

struct XWebhookSignature(String);

impl TryIntoHeaderValue for XWebhookSignature {
    type Error = actix_web::error::HttpError;

    fn try_into_value(self) -> std::result::Result<header::HeaderValue, Self::Error> {
        header::HeaderValue::from_str(&self.0).map_err(|e| e.into())
    }
}

impl header::Header for XWebhookSignature {
    fn name() -> header::HeaderName {
        header::HeaderName::from_static("x-webhook-signature")
    }

    fn parse<M: actix_web::HttpMessage>(
        msg: &M,
    ) -> std::result::Result<Self, actix_web::error::ParseError> {
        let str: std::result::Result<String, _> = from_one_raw_str(msg.headers().get(Self::name()));
        Ok(Self(str.unwrap_or_else(|_| "".to_string())))
    }
}

#[post("/sumo_api")]
pub async fn receive_sumo_api(
    req: HttpRequest,
    body: web::Bytes,
    sig: web::Header<XWebhookSignature>,
    content_type: web::Header<header::ContentType>,
    state: web::Data<AppState>,
) -> Result<impl Responder> {
    if content_type.0 != ContentType::json() {
        return Err(anyhow!("Unexpected content type: {}", content_type.0).into());
    }
    let data = match serde_json::from_slice(&body) {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to parse webhook JSON payload: {}", e);
            trace!("Request body:\n{}", String::from_utf8_lossy(&body));
            return Err(anyhow!("Failed to parse webhook JSON payload: {}", e).into());
        }
    };
    let url = state.config.url().join(req.path()).unwrap();

    let mut db = state.db.lock().unwrap();
    sumo_api::receive_webhook(
        &url,
        &body,
        &sig.deref().0,
        &data,
        &mut db,
        &state.config.webhook_secret,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}
