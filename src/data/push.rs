use std::collections::{HashMap, HashSet};

use super::{BashoId, BashoInfo, DataError, Day, DbConn, PlayerId, Rank, Result, RikishiId};
use chrono::{Duration, Utc};
use itertools::Itertools;
use rusqlite::{types::FromSqlResult, Connection, Row, RowIndex};
use url::Url;
use web_push::{
    ContentEncoding, PartialVapidSignatureBuilder, SubscriptionInfo, VapidSignatureBuilder,
    WebPushClient, WebPushError, WebPushMessageBuilder, URL_SAFE_NO_PAD,
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
    pub user_agent: String,
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

    fn from_row(row: &Row) -> rusqlite::Result<Self> {
        fn parse_json<I: RowIndex, T: serde::de::DeserializeOwned>(
            row: &Row,
            idx: I,
        ) -> FromSqlResult<T> {
            serde_json::from_slice(row.get_ref_unwrap(idx).as_bytes().unwrap())
                .map_err(|e| rusqlite::types::FromSqlError::Other(e.into()))
        }
        Ok(Self {
            id: row.get_unwrap("id"),
            player_id: row.get_unwrap("player_id"),
            info: parse_json(row, "info_json")?,
            opt_in: parse_json(row, "opt_in_json")?,
            user_agent: row.get_unwrap("user_agent"),
        })
    }

    pub fn list_all(db: &Connection) -> Result<Vec<Subscription>> {
        db.prepare(
            "
                SELECT id, player_id, info_json, opt_in_json, user_agent
                FROM player_push_subscriptions
            ",
        )?
        .query_map(params![], Self::from_row)?
        .map(|res| res.map_err(DataError::from))
        .collect()
    }

    pub fn for_player(db: &Connection, player_id: PlayerId) -> Result<Vec<Subscription>> {
        db.prepare(
            "
                SELECT id, player_id, info_json, opt_in_json, user_agent
                FROM player_push_subscriptions
                WHERE player_id = ?
            ",
        )?
        .query_map(params![player_id], Self::from_row)?
        .map(|res| res.map_err(DataError::from))
        .collect()
    }

    pub fn for_type(db: &Connection, push_type: PushTypeKey) -> Result<Vec<Subscription>> {
        Ok(Self::list_all(db)?
            .into_iter()
            .filter(|sub| sub.opt_in.contains(&push_type))
            .collect())
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
        subscriptions: &[Subscription],
        db: &DbConn,
    ) -> Result<HashMap<usize, std::result::Result<(), WebPushError>>> {
        let mut results = HashMap::new();
        let mut invalid_subscriptions = vec![];
        for sub in subscriptions {
            {
                let endpoint_url = url::Url::parse(&sub.info.endpoint);
                match endpoint_url {
                    Ok(url) => trace!(
                        "Sending push “{}—{}” to player {} on {}",
                        payload.title,
                        payload.body,
                        sub.player_id,
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

            let res = self.client.send(msg.build()?).await;
            match res {
                Ok(_) => (),
                Err(
                    web_push::WebPushError::EndpointNotValid
                    | web_push::WebPushError::EndpointNotFound,
                ) => invalid_subscriptions.push(sub.id),
                Err(ref e) => {
                    warn!(
                        "push error for player {} subscription {}: {:?}",
                        sub.player_id, sub.id, e
                    );
                } // TODO: remove subscriptions after n consecutive errors?
            }
            results.insert(sub.id, res);
        }

        if !invalid_subscriptions.is_empty() {
            debug!(
                "{}/{} subscriptions were invalid; removing from db",
                invalid_subscriptions.len(),
                subscriptions.len()
            );
            Subscription::delete(&db.lock().unwrap(), &invalid_subscriptions)?;
        }

        Ok(results)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Default)]
pub struct SendStats {
    players: usize,
    success: usize,
    invalid: usize,
    fail: usize,
}

impl SendStats {
    pub fn from_results(
        results: &HashMap<usize, std::result::Result<(), WebPushError>>,
        subscriptions: &[Subscription],
    ) -> Self {
        let mut stats = Self::default();

        let mut players = HashSet::new();
        let sub_to_player: HashMap<usize, PlayerId> = subscriptions
            .iter()
            .map(|sub| (sub.id, sub.player_id))
            .collect();
        for (id, res) in results {
            match res {
                Ok(_) => stats.success += 1,
                Err(
                    web_push::WebPushError::EndpointNotValid
                    | web_push::WebPushError::EndpointNotFound,
                ) => stats.invalid += 1,
                Err(_) => stats.fail += 1,
            };
            if players.insert(sub_to_player.get(&id).unwrap()) {
                stats.players += 1;
            }
        }

        stats
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PushType {
    Test,
    Announcement(String),
    EntriesOpen(BashoId),
    BashoStartCountdown(BashoId),
    KyujyoAlert(BashoId, RikishiId),
    DayResult(BashoId, PlayerId, Day),
    BashoResult(BashoId, PlayerId),
}

impl PushType {
    pub fn key(&self) -> PushTypeKey {
        match self {
            PushType::Test => PushTypeKey::Test,
            PushType::Announcement(_) => PushTypeKey::Announcement,
            PushType::EntriesOpen(_) => PushTypeKey::EntriesOpen,
            PushType::BashoStartCountdown(_) | PushType::KyujyoAlert(_, _) => {
                PushTypeKey::BashoStartCountdown
            }
            PushType::DayResult(_, _, _) => PushTypeKey::DayResult,
            PushType::BashoResult(_, _) => PushTypeKey::BashoResult,
        }
    }

    pub fn ttl(&self) -> Duration {
        match self {
            PushType::Test => Duration::minutes(10),
            PushType::Announcement(_) => Duration::days(1),
            PushType::EntriesOpen(_) => Duration::days(1),
            PushType::BashoStartCountdown(_basho) => Duration::hours(12),
            PushType::KyujyoAlert(_, _) => Duration::days(3),
            PushType::DayResult(_, _, _) => Duration::days(1),
            PushType::BashoResult(_, _) => Duration::days(7),
        }
    }

    pub fn subscriptions(&self, db: &Connection) -> Result<Vec<Subscription>> {
        match self {
            PushType::Test | PushType::Announcement(_) | PushType::EntriesOpen(_) => {
                Subscription::for_type(db, self.key())
            }
            PushType::BashoStartCountdown(basho_id) => {
                let procrastinators = db
                    .prepare(
                        "
                        SELECT player.id
                        FROM player
                        LEFT JOIN pick ON pick.player_id = player.id AND pick.basho_id = ?
                        GROUP BY player.id
                        HAVING COUNT(*) < 5
                    ",
                    )?
                    .query_map(params![basho_id], |row| row.get::<_, PlayerId>(0))?
                    .collect::<rusqlite::Result<HashSet<_>>>()?;
                Ok(Subscription::for_type(db, self.key())?
                    .into_iter()
                    .filter(|s| procrastinators.contains(&s.player_id))
                    .collect())
            }
            PushType::KyujyoAlert(basho_id, rikishi_id) => {
                let players = db
                    .prepare(
                        "
                        SELECT player_id
                        FROM pick
                        WHERE basho_id = ? AND rikishi_id = ?
                    ",
                    )?
                    .query_map(params![basho_id, rikishi_id], |row| {
                        row.get::<_, PlayerId>(0)
                    })?
                    .collect::<rusqlite::Result<HashSet<_>>>()?;
                Ok(Subscription::for_type(db, self.key())?
                    .into_iter()
                    .filter(|s| players.contains(&s.player_id))
                    .collect())
            }
            PushType::DayResult(_, player_id, _) => Ok(Subscription::for_player(db, *player_id)?
                .into_iter()
                .filter(|s| s.opt_in.contains(&PushTypeKey::DayResult))
                .collect()),
            PushType::BashoResult(_, _) => todo!(),
        }
    }

    pub fn build_payload(&self, base_url: &Url, db: &Connection) -> Result<Payload> {
        let url = base_url.join("pwa").unwrap().to_string();
        let payload = match self {
            PushType::Test => Payload {
                title: "Test".to_owned(),
                body: "It worked!".to_owned(),
                url,
                data: PayloadData::Empty,
            },
            PushType::Announcement(msg) => Payload {
                title: "Announcement".to_owned(),
                body: msg.to_owned(),
                url,
                data: PayloadData::Empty,
            },
            PushType::EntriesOpen(basho_id) => {
                let basho = BashoInfo::with_id(db, *basho_id)?
                    .ok_or(rusqlite::Error::QueryReturnedNoRows)?;
                Payload {
                    title: "New Basho!".to_owned(),
                    body: format!("Entries for {} are now open", basho_id),
                    url,
                    data: PayloadData::EntriesOpen {
                        basho_id: *basho_id,
                        start_date: basho.start_date.timestamp(),
                    },
                }
            }
            PushType::BashoStartCountdown(basho_id) => {
                let basho = BashoInfo::with_id(db, *basho_id)?
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
                    url,
                    data: PayloadData::BashoStartCountdown {
                        basho_id: basho.id,
                        start_date: basho.start_date.timestamp_millis(),
                    },
                }
            }
            PushType::KyujyoAlert(basho_id, rikishi_id) => {
                let (rikishi_name, rank): (String, Rank) = db.query_row(
                    "
                    SELECT family_name, rank FROM banzuke
                    WHERE basho_id = ? AND rikishi_id = ?
                ",
                    params![basho_id, rikishi_id],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                )?;
                Payload {
                    title: "Kyujyo Alert!".to_owned(),
                    body: format!(
                        "{} ({}) has gone kyujyo. You should pick another rikishi.",
                        rikishi_name, rank
                    ),
                    url,
                    data: PayloadData::Empty,
                }
            }
            PushType::DayResult(basho_id, player_id, day) => {
                let (name, score, rank, leader_score) = db.query_row(
                    "
                    SELECT
                        player.name,
                        br.wins,
                        br.rank,
                        (
                            SELECT MAX(wins)
                            FROM basho_result
                            WHERE basho_id = ?
                        )
                    FROM player
                    JOIN basho_result AS br ON br.player_id = player.id
                    WHERE player.id = ? AND br.basho_id = ?
                ",
                    params![basho_id, player_id, basho_id],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )?;
                let rikishi = db
                    .prepare(
                        "
                        SELECT
                            banzuke.family_name,
                            torikumi.win
                        FROM pick
                        NATURAL JOIN banzuke
                        LEFT NATURAL JOIN torikumi
                        WHERE
                            pick.player_id = ? AND
                            pick.basho_id = ? AND
                            torikumi.day = ?
                        ORDER BY torikumi.seq DESC
                    ",
                    )?
                    .query_map(params![player_id, basho_id, day], |row| {
                        Ok(RikishiDayResult {
                            name: row.get(0)?,
                            win: row.get(1)?,
                        })
                    })?
                    .collect::<rusqlite::Result<Vec<RikishiDayResult>>>()?;
                Payload {
                    title: format!("Day {} Results", day),
                    body: format!(
                        "{} now ranked #{}, {} points behind the leader. {}",
                        name,
                        rank,
                        leader_score - score,
                        rikishi
                            .iter()
                            .map(|r| format!(
                                "{} {}",
                                match r.win {
                                    Some(true) => "⚪️",
                                    Some(false) => "⚫️",
                                    None => "❌",
                                },
                                r.name
                            ))
                            .join(", ")
                    ),
                    url,
                    data: PayloadData::DayResult {
                        basho_id: *basho_id,
                        name,
                        day: *day,
                        rikishi,
                        rank,
                        score,
                        leader_score,
                    },
                }
            }
            PushType::BashoResult(_basho_id, _player_id) => {
                todo!()
            }
        };

        Ok(payload)
    }
}

pub async fn mass_notify_kyujyo(
    db_conn: &DbConn,
    push_builder: &PushBuilder,
    url: &Url,
    basho_id: BashoId,
) -> Result<()> {
    let rikishi;
    {
        let db = db_conn.lock().unwrap();
        rikishi = db
            .prepare(
                "
            SELECT rikishi_id, family_name
            FROM banzuke
            WHERE basho_id = ? AND kyujyo = 1
        ",
            )?
            .query_map(params![basho_id], |row| Ok((row.get(0)?, row.get(1)?)))?
            .collect::<rusqlite::Result<Vec<(RikishiId, String)>>>()?;
    }
    for (rikishi_id, rikishi_name) in rikishi {
        info!(
            "Building kyujyo notice for {} (id {})",
            rikishi_name, rikishi_id
        );
        let payload;
        let ttl;
        let subscriptions;
        {
            let push_type = PushType::KyujyoAlert(basho_id, rikishi_id);
            let db = db_conn.lock().unwrap();
            payload = push_type.build_payload(url, &db)?;
            ttl = push_type.ttl();
            subscriptions = push_type.subscriptions(&db)?;
        }
        debug!(
            "Notifying {} devices for kyujyo rikishi {}",
            subscriptions.len(),
            rikishi_name
        );
        let results = push_builder
            .clone()
            .send(payload, ttl, &subscriptions, db_conn)
            .await?;
        let stats = SendStats::from_results(&results, &subscriptions);
        info!("{:?}", stats);
    }

    Ok(())
}

pub async fn mass_notify_day_result(
    db_conn: &DbConn,
    push_builder: &PushBuilder,
    url: &Url,
    basho_id: BashoId,
    day: u8,
) -> Result<()> {
    let player_ids;
    {
        let db = db_conn.lock().unwrap();
        player_ids = db
            .prepare(
                "
            SELECT player_id
            FROM basho_result
            WHERE basho_id = ?
        ",
            )?
            .query_map(params![basho_id], |row| row.get::<_, PlayerId>(0))?
            .collect::<rusqlite::Result<Vec<PlayerId>>>()?;
    }
    info!(
        "Sending day {} results to {} possible players",
        day,
        player_ids.len()
    );
    for (i, player_id) in player_ids.iter().enumerate() {
        let payload;
        let subscriptions;
        let ttl;
        {
            let db = db_conn.lock().unwrap();
            let push_type = PushType::DayResult(basho_id, *player_id, day);
            subscriptions = push_type.subscriptions(&db)?;
            if subscriptions.is_empty() {
                continue;
            }
            payload = push_type.build_payload(url, &db)?;
            ttl = push_type.ttl();
        }
        trace!(
            "Notifying player {}/{} id {} with {} subscriptions: {}",
            i + 1,
            player_ids.len(),
            player_id,
            subscriptions.len(),
            payload.body
        );
        let results = push_builder
            .clone()
            .send(payload, ttl, &subscriptions, db_conn)
            .await?;
        let stats = SendStats::from_results(&results, &subscriptions);
        info!("{:?}", stats);
    }
    Ok(())
}

// Keep in sync with service-worker.ts

#[derive(Debug, Serialize)]
pub struct Payload {
    title: String,
    body: String,
    url: String,
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
        rikishi: Vec<RikishiDayResult>,
        rank: u16,
        score: u8,
        leader_score: u8,
    },
}

#[derive(Debug, Serialize)]
struct RikishiDayResult {
    name: String,
    win: Option<bool>,
}
