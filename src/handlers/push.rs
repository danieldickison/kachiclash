use crate::data::push::{PushType, Subscription};
use crate::data::Player;
use crate::handlers::HandlerError;
use crate::AppState;
use actix_identity::Identity;
use actix_web::{post, web, HttpResponse, Responder};
// use serde::{Deserialize, Deserializer};
use web_push::SubscriptionInfo;

use super::{IdentityExt, Result};

#[post("/check")]
pub async fn check(
    subscription: web::Json<SubscriptionInfo>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<impl Responder> {
    let player_id = identity.require_player_id()?;
    let db = state.db.lock().unwrap();
    for sub in Subscription::for_player(&db, player_id)? {
        if sub.info == subscription.0 {
            debug!("Matched player {} subscription {}", player_id, sub.id);
            return Ok(web::Json(sub));
        }
    }
    Err(HandlerError::NotFound("subscription".to_string()))
}

#[post("/test")]
pub async fn test(state: web::Data<AppState>, identity: Identity) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let push_type = PushType::Test;
    let payload;
    let subs;
    {
        let db = state.db.lock().unwrap();
        subs = Subscription::for_player(&db, player_id)?;
        if subs.is_empty() {
            return Err(super::HandlerError::NotFound(
                "push subscription".to_owned(),
            ));
        }
        payload = push_type.build_payload(&state.config.url(), &db)?;
    }

    state
        .push
        .clone()
        .send(payload, push_type.ttl(), &subs, &state.db)
        .await?;

    Ok(HttpResponse::Ok().finish())
}

#[post("/trigger")]
pub async fn trigger(
    state: web::Data<AppState>,
    identity: Identity,
    data: web::Json<PushType>,
) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let payload;
    let subscriptions;
    let ttl;
    {
        let db = state.db.lock().unwrap();
        let player = Player::with_id(&db, player_id)?;
        if !player.map_or(false, |p| p.is_admin()) {
            return Err(HandlerError::MustBeLoggedIn);
        }
        payload = data.build_payload(&state.config.url(), &db)?;
        subscriptions = Subscription::for_type(&db, data.key())?;
        ttl = data.ttl();
    }

    state
        .push
        .clone()
        .send(payload, ttl, &subscriptions, &state.db)
        .await?;

    Ok(HttpResponse::Created().finish())
}
