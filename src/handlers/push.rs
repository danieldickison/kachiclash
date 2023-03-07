use crate::data::push::{self, PushType};
use crate::data::BashoInfo;
use crate::AppState;
use actix_identity::Identity;
use actix_web::{web, HttpResponse};
// use serde::{Deserialize, Deserializer};
use web_push::SubscriptionInfo;

use super::{IdentityExt, Result, UserAgent};

pub async fn register(
    subscription: web::Json<SubscriptionInfo>,
    user_agent: web::Header<UserAgent>,
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let db = state.db.lock().unwrap();
    push::add_player_subscription(&db, player_id, subscription.0, &user_agent.0.to_string())?;
    Ok(HttpResponse::Ok().finish())
}

pub async fn test(state: web::Data<AppState>, identity: Identity) -> Result<HttpResponse> {
    let player_id = identity.require_player_id()?;
    let push_type;
    let payload;
    let subs;
    {
        let db = state.db.lock().unwrap();
        subs = push::subscriptions_for_player(&db, player_id)?;
        let (current, _prev) = BashoInfo::current_and_previous(&db)?;
        if let Some(basho) = current {
            push_type = PushType::EntriesOpen(basho);
            payload = push_type.build_payload(&db)?;
        } else {
            return Err(super::HandlerError::NotFound(
                "push subscription".to_string(),
            ));
        }
    }

    state
        .push
        .clone()
        .send(payload, push_type.ttl(), subs, &state.db)
        .await?;

    Ok(HttpResponse::Ok().finish())
}
