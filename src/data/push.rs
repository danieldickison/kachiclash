use std::collections::HashSet;

use super::{BashoId, BashoInfo, DataError, Day, DbConn, PlayerId, Result};
use chrono::{Duration, Utc};
use rusqlite::Connection;
use web_push::{
    ContentEncoding, PartialVapidSignatureBuilder, SubscriptionInfo, VapidSignatureBuilder,
    WebPushClient, WebPushMessageBuilder, URL_SAFE_NO_PAD,
};

#[derive(Debug, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum PushTypeKey {
    Test,
    EntriesOpen,
    BashoStartCountdown,
    DayResult,
    BashoResult,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Subscription {
    pub id: usize,
    pub info: SubscriptionInfo,
}

pub fn add_player_subscription(
    db: &Connection,
    player_id: PlayerId,
    subscription: SubscriptionInfo,
    opt_in: HashSet<PushTypeKey>,
    user_agent: &str,
) -> Result<()> {
    info!(
        "Registering push subscription for player {}, user agent: {}, opt-in: {:?}",
        player_id, user_agent, opt_in
    );
    db.prepare(
        "
            INSERT INTO player_push_subscriptions
                (player_id, info_json, user_agent, opt_in_json)
            VALUES (?, ?, ?, ?)
            ON CONFLICT (info_json) DO UPDATE
                player_id = excluded.player_id,
                user_agent = excluded.user_agent,
                opt_in_json = excluded.opt_in_json
        ",
    )?
    .execute(params![
        player_id,
        serde_json::to_string(&subscription)?,
        user_agent,
        serde_json::to_string(&opt_in)?,
    ])?;
    Ok(())
}

pub fn delete_subscriptions(db: &Connection, sub_ids: &[usize]) -> Result<()> {
    let mut stmt = db.prepare(
        "
        DELETE FROM player_push_subscriptions
        WHERE id = ?
    ",
    )?;
    for id in sub_ids {
        println!("Removing push subscription {}", id);
        if let Err(e) = stmt.execute(params![id]) {
            warn!("Failed to delete subscription {}: {}", id, e);
        }
    }
    Ok(())
}

pub fn subscriptions_for_player(db: &Connection, player_id: PlayerId) -> Result<Vec<Subscription>> {
    db.prepare(
        "
            SELECT id, info_json
            FROM player_push_subscriptions AS sub
            WHERE player_id = ?
        ",
    )?
    .query_map(params![player_id], |row| {
        let id = row.get::<_, usize>(0)?;
        let json = row.get::<_, String>(1)?;
        Ok((id, json))
    })?
    .map(|row| {
        row.map_or_else(
            |err| Err(DataError::DatabaseError(err)),
            |(id, json)| {
                Ok(Subscription {
                    id,
                    info: serde_json::from_str(&json)?,
                })
            },
        )
    })
    // convert Vec<Result<..>> to Result<Vec<..>>
    .collect::<Vec<Result<Subscription>>>()
    .into_iter()
    .collect()
}

#[derive(Clone)]
pub struct PushBuilder {
    vapid: PartialVapidSignatureBuilder,
    client: WebPushClient,
}

impl PushBuilder {
    pub fn with_base64_private_key(base64: &str) -> Result<Self> {
        Ok(Self {
            vapid: VapidSignatureBuilder::from_base64_no_sub(base64, URL_SAFE_NO_PAD)?,
            client: WebPushClient::new()?,
        })
    }

    pub async fn send(
        self,
        payload: Payload,
        ttl: Duration,
        subscriptions: Vec<Subscription>,
        db: &DbConn,
    ) -> Result<()> {
        let mut invalid_subscriptions = vec![];
        for sub in &subscriptions {
            {
                let endpoint_url = url::Url::parse(&sub.info.endpoint);
                match endpoint_url {
                    Ok(url) => trace!(
                        "Sending push {:?} to {}",
                        payload.title,
                        url.host_str().unwrap_or("<invalid host>")
                    ),
                    Err(e) => warn!("endpoint url parse error {}", e),
                };
            }

            let mut msg = WebPushMessageBuilder::new(&sub.info)?;
            msg.set_ttl(ttl.num_seconds() as u32);

            let payload_json = serde_json::to_vec(&payload)?;
            msg.set_payload(ContentEncoding::Aes128Gcm, &payload_json);

            let mut sig = self.vapid.clone().add_sub_info(&sub.info);
            // sub appears to be necessary for Firefox and Safari
            sig.add_claim("sub", "https://kachiclash.com");
            msg.set_vapid_signature(sig.build()?);

            match self.client.send(msg.build()?).await {
                Ok(_) => (),
                Err(
                    web_push::WebPushError::EndpointNotValid
                    | web_push::WebPushError::EndpointNotFound,
                ) => invalid_subscriptions.push(sub.id),
                Err(e) => warn!("push error {:?}", e),
                // TODO: remove subscriptions after n consecutive errors?
            }
        }

        if !invalid_subscriptions.is_empty() {
            debug!(
                "{}/{} subscriptions were invalid; removing from db",
                invalid_subscriptions.len(),
                subscriptions.len()
            );
            delete_subscriptions(&db.lock().unwrap(), &invalid_subscriptions)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum PushType {
    Test,
    EntriesOpen(BashoInfo),
    BashoStartCountdown(BashoInfo),
    DayResult(BashoInfo, PlayerId, Day),
    BashoResult(BashoInfo, PlayerId),
}

impl PushType {
    pub fn key(&self) -> PushTypeKey {
        match self {
            PushType::Test => PushTypeKey::Test,
            PushType::EntriesOpen(_) => PushTypeKey::EntriesOpen,
            PushType::BashoStartCountdown(_) => PushTypeKey::BashoStartCountdown,
            PushType::DayResult(_, _, _) => PushTypeKey::DayResult,
            PushType::BashoResult(_, _) => PushTypeKey::BashoResult,
        }
    }

    pub fn ttl(&self) -> Duration {
        match self {
            PushType::Test => Duration::minutes(10),
            PushType::EntriesOpen(_) => Duration::days(1),
            PushType::BashoStartCountdown(basho) => Duration::max(
                Duration::hours(1),
                basho.start_date.signed_duration_since(Utc::now()),
            ),
            PushType::DayResult(_, _, _) => Duration::days(1),
            PushType::BashoResult(_, _) => Duration::days(7),
        }
    }

    pub fn build_payload(&self, _db: &Connection) -> Result<Payload> {
        let payload = match self {
            PushType::Test => Payload {
                title: "Test".to_owned(),
                body: "It works!".to_owned(),
                data: PayloadData::Test {
                    foo: "this is a test".to_owned(),
                },
            },
            PushType::EntriesOpen(basho) => Payload {
                title: "New Basho!".to_owned(),
                body: format!("Entries for {} are now open", basho.id),
                data: PayloadData::EntriesOpen {
                    basho_id: basho.id,
                    start_date: basho.start_date.timestamp(),
                },
            },
            PushType::BashoStartCountdown(_) => todo!(),
            PushType::DayResult(_, _, _) => todo!(),
            PushType::BashoResult(_, _) => todo!(),
        };

        Ok(payload)
    }
}

// Keep in sync with service-worker.ts

#[derive(Debug, Serialize)]
pub struct Payload {
    title: String,
    body: String,
    #[serde(flatten)]
    data: PayloadData,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum PayloadData {
    Test {
        foo: String,
    },

    EntriesOpen {
        basho_id: BashoId,
        start_date: i64,
    },

    BashoStartCountdown {
        basho_id: BashoId,
        start_date: i64,
    },

    DayResult {
        basho_id: BashoId,
        name: String,
        day: Day,
        rikishi: [RikishiDayResult; 5],
        rank: u16,
        leaders: Vec<String>,
        leader_score: u8,
    },
}

#[derive(Debug, Serialize)]
struct RikishiDayResult {
    name: String,
    won: Option<bool>,
    against: Option<String>,
    wins: u8,
    losses: u8,
    absence: u8,
}
