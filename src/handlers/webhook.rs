use std::ops::Deref;

use actix_identity::Identity;
use actix_web::http::header::{self, from_one_raw_str, ContentType, TryIntoHeaderValue};
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use anyhow::anyhow;

use super::{BaseTemplate, Result};
use crate::data::push::mass_notify_day_result;
use crate::external::sumo_api;
use crate::AppState;

#[post("/register")]
pub async fn register(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    BaseTemplate::for_admin(&state.db.lock().unwrap(), &identity, &state)?;
    Ok(sumo_api::register_webhook(&state.config).await?)
}

#[post("/delete")]
pub async fn delete(state: web::Data<AppState>, identity: Identity) -> Result<impl Responder> {
    BaseTemplate::for_admin(&state.db.lock().unwrap(), &identity, &state)?;
    Ok(sumo_api::delete_webhook(&state.config).await?)
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
    Ok(sumo_api::request_webhook_test(&state.config, &query.webhook_type).await?)
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

    let sumo_api::ReceiveWebhookResult {
        basho_id,
        day,
        should_send_notifications,
    } = sumo_api::receive_webhook(
        &url,
        &body,
        &sig.deref().0,
        &data,
        &mut state.db.lock().unwrap(),
        &state.config.webhook_secret,
    )?;
    if should_send_notifications {
        let stats =
            mass_notify_day_result(&state.db, &state.push, &state.config.url(), basho_id, day)
                .await?;
        info!("push notifications sent: {:?}", stats);
    }
    Ok(HttpResponse::NoContent().finish())
}
