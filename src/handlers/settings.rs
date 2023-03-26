use std::collections::HashSet;

use actix_identity::Identity;
use actix_web::{get, post, web, HttpResponse, Responder};
use anyhow::anyhow;
use askama::Template;
use rusqlite::Connection;

use super::user_agent::UserAgent;
use super::{BaseTemplate, HandlerError, Result};
use crate::data::player::{self, Player, PlayerId};
use crate::data::push::{self, PushTypeKey};
use crate::handlers::IdentityExt;
use crate::AppState;

#[derive(Template)]
#[template(path = "settings.html")]
pub struct SettingsTemplate {
    base: BaseTemplate,
}

#[derive(Debug, Deserialize)]
pub struct FormData {
    name: String,
    push_subscription: Option<web_push::SubscriptionInfo>,
    notification_opt_in: HashSet<PushTypeKey>,
}

#[get("/settings")]
pub async fn settings_page(
    state: web::Data<AppState>,
    identity: Identity,
) -> Result<SettingsTemplate> {
    let db = state.db.lock().unwrap();
    let base = BaseTemplate::new(&db, &identity, &state)?;
    if base.player.is_some() {
        Ok(SettingsTemplate { base })
    } else {
        Err(HandlerError::MustBeLoggedIn)
    }
}

#[post("/settings")]
pub async fn settings_post(
    form: web::Form<FormData>,
    state: web::Data<AppState>,
    user_agent: web::Header<UserAgent>,
    identity: Identity,
) -> Result<impl Responder> {
    trace!("in settings_post with form {:?}", form.0);
    let player_id = identity.require_player_id()?;
    let mut db = state.db.lock().unwrap();
    match settings_post_inner(&mut db, player_id, form.0, user_agent.0).await {
        Ok(_) => Ok(HttpResponse::Accepted().finish()),
        Err(e) => {
            warn!("settings_post fail: {:?}", e);
            Ok(HttpResponse::InternalServerError().body(e.to_string()))
        }
    }
}

async fn settings_post_inner(
    db: &mut Connection,
    player_id: PlayerId,
    form: FormData,
    user_agent: UserAgent,
) -> anyhow::Result<()> {
    if !Player::name_is_valid(&form.name) {
        return Err(anyhow!("Invalid name: {}", form.name));
    }

    let txn = db.transaction()?;

    if let Some(subscription) = form.push_subscription {
        push::add_player_subscription(
            &txn,
            player_id,
            &subscription,
            &form.notification_opt_in,
            &user_agent.to_string(),
        )?;
    }

    Player::set_name(&txn, player_id, &form.name)?;

    txn.commit()?;
    Ok(())
}
