use std::collections::HashSet;

use super::{BashoId, BashoInfo, DataError, Day, DbConn, PlayerId, Result};
use chrono::{Duration, Utc};
use rusqlite::Connection;
use web_push::{
    ContentEncoding, PartialVapidSignatureBuilder, SubscriptionInfo, VapidSignatureBuilder,
    WebPushClient, WebPushMessageBuilder, URL_SAFE_NO_PAD,
};

// Keep types in sync with push.ts
#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize, Serialize)]
pub enum PushTypeKey {
    Test,
    Announcement,
    EntriesOpen,
    BashoStartCountdown,
    DayResult,
    BashoResult,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Subscription {
    pub id: usize,
    pub player_id: PlayerId,
    pub info: SubscriptionInfo,
    pub opt_in: HashSet<PushTypeKey>,
}

impl Subscription {
    pub fn register(
        db: &Connection,
        player_id: PlayerId,
        subscription: &SubscriptionInfo,
        opt_in: &HashSet<PushTypeKey>,
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
            ON CONFLICT (info_json) DO UPDATE SET
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

    pub fn delete(db: &Connection, sub_ids: &[usize]) -> Result<()> {
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

    pub fn for_player(db: &Connection, player_id: PlayerId) -> Result<Vec<Subscription>> {
        db.prepare(
            "
            SELECT id, info_json, opt_in_json
            FROM player_push_subscriptions AS sub
            WHERE player_id = ?
        ",
        )?
        .query_map(params![player_id], |row| {
            let id = row.get::<_, usize>(0)?;
            let info = row.get::<_, String>(1)?;
            let opt_in = row.get::<_, String>(2)?;
            Ok((id, info, opt_in))
        })?
        .map(|row| {
            row.map_or_else(
                |err| Err(DataError::DatabaseError(err)),
                |(id, info, opt_in)| {
                    Ok(Subscription {
                        id,
                        player_id,
                        info: serde_json::from_str(&info)?,
                        opt_in: serde_json::from_str(&opt_in)?,
                    })
                },
            )
        })
        // convert Vec<Result<..>> to Result<Vec<..>>
        .collect::<Vec<Result<Subscription>>>()
        .into_iter()
        .collect()
    }

    pub fn for_type(db: &Connection, push_type: PushTypeKey) -> Result<Vec<Subscription>> {
        db.prepare(
            "
            SELECT id, player_id, info_json, opt_in_json
            FROM player_push_subscriptions AS sub
        ",
        )?
        .query_map(params![], |row| {
            let id = row.get::<_, usize>(0)?;
            let player_id = row.get::<_, PlayerId>(1)?;
            let info = row.get::<_, String>(2)?;
            let opt_in = row.get::<_, String>(3)?;
            Ok((id, player_id, info, opt_in))
        })?
        .map(|row| {
            row.map_or_else(
                |err| Err(DataError::DatabaseError(err)),
                |(id, player_id, info, opt_in)| {
                    Ok(Subscription {
                        id,
                        player_id,
                        info: serde_json::from_str(&info)?,
                        opt_in: serde_json::from_str(&opt_in)?,
                    })
                },
            )
        })
        .filter(|res| {
            res.as_ref()
                .map(|sub| sub.opt_in.contains(&push_type))
                .unwrap_or(false)
        })
        // convert Vec<Result<..>> to Result<Vec<..>>
        .collect::<Vec<Result<Subscription>>>()
        .into_iter()
        .collect()
    }
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
        trace!(
            "sending “{}” to {} subscriptions",
            payload.title,
            subscriptions.len()
        );
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
            Subscription::delete(&db.lock().unwrap(), &invalid_subscriptions)?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PushType {
    Test,
    Announcement(String),
    EntriesOpen(BashoId),
    BashoStartCountdown(BashoId),
    DayResult(BashoId, PlayerId, Day),
    BashoResult(BashoId, PlayerId),
}

impl PushType {
    pub fn key(&self) -> PushTypeKey {
        match self {
            PushType::Test => PushTypeKey::Test,
            PushType::Announcement(_) => PushTypeKey::Announcement,
            PushType::EntriesOpen(_) => PushTypeKey::EntriesOpen,
            PushType::BashoStartCountdown(_) => PushTypeKey::BashoStartCountdown,
            PushType::DayResult(_, _, _) => PushTypeKey::DayResult,
            PushType::BashoResult(_, _) => PushTypeKey::BashoResult,
        }
    }

    pub fn ttl(&self) -> Duration {
        match self {
            PushType::Test => Duration::minutes(10),
            PushType::Announcement(_) => Duration::days(1),
            PushType::EntriesOpen(_) => Duration::days(1),
            PushType::BashoStartCountdown(basho) => Duration::hours(12),
            PushType::DayResult(_, _, _) => Duration::days(1),
            PushType::BashoResult(_, _) => Duration::days(7),
        }
    }

    pub fn build_payload(&self, db: &Connection) -> Result<Payload> {
        let payload = match self {
            PushType::Test => Payload {
                title: "Test".to_owned(),
                body: "It worked!".to_owned(),
                data: PayloadData::Empty,
            },
            PushType::Announcement(msg) => Payload {
                title: "Announcement".to_owned(),
                body: msg.to_owned(),
                data: PayloadData::Empty,
            },
            PushType::EntriesOpen(basho_id) => {
                let basho = BashoInfo::with_id(&db, *basho_id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                Payload {
                    title: "New Basho!".to_owned(),
                    body: format!("Entries for {} are now open", basho_id),
                    data: PayloadData::EntriesOpen {
                        basho_id: *basho_id,
                        start_date: basho.start_date.timestamp(),
                    },
                }
            }
            PushType::BashoStartCountdown(basho_id) => {
                let basho = BashoInfo::with_id(&db, *basho_id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                let duration = basho.start_date.signed_duration_since(Utc::now());
                let body = if duration > Duration::days(2) {
                    format!(
                        "You have {} days to get your picks in!",
                        duration.num_days()
                    )
                } else if duration > Duration::days(1) {
                    "You have one day to get your picks in!".to_owned()
                } else {
                    format!(
                        "You have {} hours to get your picks in!",
                        duration.num_hours()
                    )
                };
                Payload {
                    title: "Basho Reminder".to_owned(),
                    body,
                    data: PayloadData::BashoStartCountdown {
                        basho_id: basho.id,
                        start_date: basho.start_date.timestamp_millis(),
                    },
                }
            }
            PushType::DayResult(basho_id, player_id, day) => {
                todo!()
            }
            PushType::BashoResult(basho_id, player_id) => {
                todo!()
            }
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
    Empty,

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
