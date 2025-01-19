use actix_identity::Identity;
use actix_web::{post, web, HttpRequest, HttpResponse, Responder};
use anyhow::anyhow;

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

#[post("/sumo_api")]
pub async fn receive_sumo_api(
    req: HttpRequest,
    body: web::Bytes,
    state: web::Data<AppState>,
) -> Result<impl Responder> {
    let sig = req
        .headers()
        .get("X-Webhook-Signature")
        .map(|h| {
            debug!("Received signature: {:?}", h);
            h
        })
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing X-Webhook-Signature"))?
        .to_str()
        .map_err(|_e| actix_web::error::ErrorBadRequest("Malformed X-Webhook-Signature"))?;

    let data = match serde_json::from_slice(&body) {
        Ok(data) => data,
        Err(e) => {
            warn!("Failed to parse webhook JSON payload: {}", e);
            trace!("Request body:\n{}", String::from_utf8_lossy(&body));
            return Err(anyhow!("Failed to parse webhook JSON payload: {}", e).into());
        }
    };

    let mut db = state.db.lock().unwrap();
    sumo_api::receive_webhook(
        &req.full_url(),
        &body,
        &sig,
        &data,
        &mut db,
        &state.config.webhook_secret,
    )
    .await?;
    Ok(HttpResponse::NoContent().finish())
}
