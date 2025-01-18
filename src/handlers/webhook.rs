use actix_web::{post, web, HttpRequest, HttpResponse, Responder};

use super::Result;
use crate::external::sumo_api;
use crate::AppState;

#[post("/register")]
pub async fn register(state: web::Data<AppState>) -> Result<impl Responder> {
    sumo_api::register_webhook(&state.config).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/test")]
pub async fn request_test(
    state: web::Data<AppState>,
    webhook_type: web::Query<String>,
) -> Result<impl Responder> {
    sumo_api::request_webhook_test(&state.config, &webhook_type.0).await?;
    Ok(HttpResponse::NoContent().finish())
}

#[post("/sumo_api")]
pub async fn receive_sumo_api(
    req: HttpRequest,
    data: web::Json<sumo_api::MatchResultsWebhookData>,
    state: web::Data<AppState>,
) -> Result<impl Responder> {
    let sig = req
        .headers()
        .get("X-Sumo-Webhook-Signature")
        .ok_or_else(|| actix_web::error::ErrorBadRequest("Missing X-Sumo-Webhook-Signature"))?
        .to_str()
        .map_err(|e| actix_web::error::ErrorBadRequest("Malformed X-Sumo-Webhook-Signature"))?;
    let mut db = state.db.lock().unwrap();
    sumo_api::receive_webhook(&sig, &data, &mut db, &state.config.webhook_secret).await?;
    Ok(HttpResponse::NoContent().finish())
}
