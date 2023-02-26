use super::{BashoId, BashoInfo, DataError, Day, PlayerId, Result};
use chrono::{Duration, Utc};
use rusqlite::Connection;
use std::fs::File;
use web_push::{
    ContentEncoding, PartialVapidSignatureBuilder, SubscriptionInfo, VapidSignatureBuilder,
    WebPushClient, WebPushMessageBuilder,
};

pub fn add_player_subscription(
    db: &Connection,
    player_id: PlayerId,
    subscription: SubscriptionInfo,
    user_agent: &str,
) -> Result<()> {
    println!(
        "Registering push subscription for player {}, user agent: {}",
        player_id, user_agent
    );
    db.prepare(
        "
            INSERT INTO player_push_subscriptions
            (player_id, info_json, user_agent)
            VALUES (?, ?, ?)
        ",
    )?
    .execute(params![
        player_id,
        serde_json::to_string(&subscription)?,
        user_agent
    ])?;
    Ok(())
}

pub fn subscriptions_for_player(
    db: &Connection,
    player_id: PlayerId,
) -> Result<Vec<SubscriptionInfo>> {
    db.prepare(
        "
            SELECT info_json
            FROM player_push_subscriptions AS sub
            WHERE player_id = ?
        ",
    )?
    .query_map(params![player_id], |row| row.get::<_, String>(0))?
    .map(|row| {
        row.map_or_else(
            |err| Err(DataError::DatabaseError(err)),
            |json| serde_json::from_str(&json).map_err(DataError::from),
        )
    })
    // convert Vec<Result<..>> to Result<Vec<..>>
    .collect::<Vec<Result<SubscriptionInfo>>>()
    .into_iter()
    .collect()
}

#[derive(Clone)]
pub struct PushBuilder {
    vapid: PartialVapidSignatureBuilder,
    client: WebPushClient,
}

impl PushBuilder {
    pub fn with_pem(path: &str) -> Result<Self> {
        Ok(Self {
            vapid: VapidSignatureBuilder::from_pem_no_sub(
                File::open(path).expect("push key pem file"),
            )?,
            client: WebPushClient::new()?,
        })
    }

    pub async fn send(
        self,
        payload: Payload,
        ttl: Duration,
        subscriptions: Vec<SubscriptionInfo>,
    ) -> Result<()> {
        let mut invalid_subscriptions = vec![];
        for sub in &subscriptions {
            let mut msg = WebPushMessageBuilder::new(&sub)?;
            msg.set_ttl(ttl.num_seconds() as u32);
            let payload_json = serde_json::to_vec(&payload)?;
            msg.set_payload(ContentEncoding::Aes128Gcm, &payload_json);
            let sig = self.vapid.clone().add_sub_info(&sub).build()?;
            msg.set_vapid_signature(sig);
            match self.client.send(msg.build()?).await {
                Ok(_) => (),
                Err(
                    web_push::WebPushError::EndpointNotValid
                    | web_push::WebPushError::EndpointNotFound,
                ) => invalid_subscriptions.push(sub),
                Err(e) => warn!("push error {}", e),
            }
        }

        if !invalid_subscriptions.is_empty() {
            debug!(
                "{}/{} subscriptions were invalid; removing from db",
                invalid_subscriptions.len(),
                subscriptions.len()
            );
            todo!("remove push subscriptions");
        }

        Ok(())
    }
}

#[derive(Debug)]
pub enum PushType {
    EntriesOpen(BashoInfo),
    BashoStartCountdown(BashoInfo),
    DayResult(BashoInfo, PlayerId, Day),
    BashoResult(BashoInfo, PlayerId),
}

impl PushType {
    pub fn ttl(&self) -> Duration {
        match self {
            PushType::EntriesOpen(_) => Duration::days(1),
            PushType::BashoStartCountdown(basho) => Duration::max(
                Duration::hours(1),
                basho.start_date.signed_duration_since(Utc::now()),
            ),
            PushType::DayResult(_, _, _) => Duration::days(1),
            PushType::BashoResult(_, _) => Duration::days(7),
        }
    }

    pub fn build_payload(&self, db: &Connection) -> Result<Payload> {
        let payload = match self {
            PushType::EntriesOpen(basho) => Payload {
                msg: format!("Entries for {} are now open!", basho.id),
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
    msg: String,
    #[serde(flatten)]
    data: PayloadData,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum PayloadData {
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
