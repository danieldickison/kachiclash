use std::collections::HashSet;

use crate::data::push::{self, PushType, PushTypeKey};
use crate::AppState;
use actix_identity::Identity;
use actix_web::{post, web, HttpResponse};
// use serde::{Deserialize, Deserializer};
use web_push::SubscriptionInfo;

use super::{IdentityExt, Result, UserAgent};

#[derive(Debug, Serialize, Deserialize)]
pub struct RegisterParams {
    subscription: SubscriptionInfo,
    opt_in: HashSet<PushTypeKey>,
}

#[post("/register")]
pub async fn register(
    params: web::Json<RegisterParams>,
    user_agent: web::Header<UserAgent>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let db = state.db.lock().unwrap();
    push::add_player_subscription(
        &db,
        player_id,
        params.0.subscription,
        params.0.opt_in,
        &user_agent.0.to_string(),
    )?;
    Ok(HttpResponse::Ok().finish())
}

#[post("/check")]
pub async fn check(
    subscription: web::Json<SubscriptionInfo>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let db = state.db.lock().unwrap();
    for sub in push::subscriptions_for_player(&db, player_id)? {
        if sub.info == subscription.0 {
            debug!("Matched player {} subscription {}", player_id, sub.id);
            return Ok(HttpResponse::Ok().finish());
        }
    }
    Ok(HttpResponse::NotFound().finish())
}

#[post("/test")]
pub async fn test(state: web::Data<AppState>, identity: Identity) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let push_type = PushType::Test;
    let payload;
    let subs;
    {
        let db = state.db.lock().unwrap();
        subs = push::subscriptions_for_player(&db, player_id)?;
        if subs.is_empty() {
            return Err(super::HandlerError::NotFound(
                "push subscription".to_owned(),
            ));
        }
        payload = push_type.build_payload(&db)?;
    }

    state
        .push
        .clone()
        .send(payload, push_type.ttl(), subs, &state.db)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
