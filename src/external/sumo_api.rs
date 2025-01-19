use std::collections::HashMap;
use std::sync::LazyLock;
use std::{collections::HashSet, time::Duration};

use anyhow::{bail, Result};
use chrono::{DateTime, Utc};
use hmac_sha256::HMAC;
use itertools::Itertools;
use rusqlite::Connection;
use url::Url;

use crate::data::{
    basho::{update_torikumi, TorikumiMatchUpdateData},
    BashoId, Rank, RankDivision,
};
use crate::data::{BashoInfo, DbConn};
use crate::Config;

const CONNECTION_TIMEOUT: u64 = 10;
const RESPONSE_TIMEOUT: u64 = 20;
static DRY_RUN: LazyLock<bool> =
    LazyLock::new(|| std::env::var("SUMO_API_DRY_RUN").ok() == Some("1".to_string()));

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanzukeResponse {
    #[allow(dead_code)]
    pub basho_id: BashoId,
    #[allow(dead_code)]
    pub division: RankDivision,
    pub east: Vec<RikishiResponse>,
    pub west: Vec<RikishiResponse>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RikishiResponse {
    pub shikona_en: String,
    pub rank: Rank,
    #[serde(default)] // newly retired rikishi might be missing this field
    pub record: Vec<BoutResponse>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BoutResponse {
    pub result: BoutResult,
    pub opponent_shikona_en: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum BoutResult {
    Win,
    Loss,
    Absent,

    #[serde(rename = "fusen loss")]
    FusenLoss,

    #[serde(rename = "fusen win")]
    FusenWin,

    #[serde(rename = "")]
    None,
}

impl BanzukeResponse {
    pub async fn fetch(basho_id: BashoId, division: RankDivision) -> Result<Self, reqwest::Error> {
        let url = format!(
            "https://www.sumo-api.com/api/basho/{basho}/banzuke/{division}",
            basho = basho_id.id()
        );
        debug!("sending request to {}", url);
        make_client()?.get(&url).send().await?.json().await
    }

    pub fn day_complete(&self, day: u8) -> bool {
        let day_idx = day as usize - 1;
        assert!(day_idx < 15);
        self.all_rikishi().all(|rikishi| {
            rikishi
                .record
                .get(day_idx)
                .map_or(false, |res| res.result != BoutResult::None)
        })
    }

    pub fn torikumi_update_data(&self, day: u8) -> Vec<TorikumiMatchUpdateData> {
        let day_idx = day as usize - 1;
        assert!(day_idx < 15);
        let mut seen_rikishi = HashSet::new();
        let mut out = vec![];
        for rikishi in self.all_rikishi() {
            if seen_rikishi.contains(&rikishi.shikona_en) {
                continue;
            }

            let torikumi = &rikishi.record[day_idx];
            assert!(seen_rikishi.insert(&rikishi.shikona_en));
            if !torikumi.opponent_shikona_en.is_empty() {
                assert!(seen_rikishi.insert(&torikumi.opponent_shikona_en));
            }

            match torikumi {
                BoutResponse {
                    result: BoutResult::Win | BoutResult::FusenWin,
                    opponent_shikona_en,
                } => out.push(TorikumiMatchUpdateData {
                    winner: rikishi.shikona_en.to_owned(),
                    loser: opponent_shikona_en.to_owned(),
                }),
                BoutResponse {
                    result: BoutResult::Loss | BoutResult::FusenLoss,
                    opponent_shikona_en,
                } => out.push(TorikumiMatchUpdateData {
                    winner: opponent_shikona_en.to_owned(),
                    loser: rikishi.shikona_en.to_owned(),
                }),
                BoutResponse {
                    result: BoutResult::Absent,
                    ..
                } => (), // expected and normal
                BoutResponse {
                    result: BoutResult::None,
                    ..
                } => warn!(
                    "Unexpected missing result for {} day {}",
                    rikishi.shikona_en, day
                ),
            }
        }
        out
    }

    pub fn all_rikishi(&self) -> impl Iterator<Item = &RikishiResponse> {
        self.east.iter().chain(self.west.iter())
    }
}

pub async fn query_and_update_sumo_api_torikumi(
    basho_id: BashoId,
    day: u8,
    db_conn: &DbConn,
) -> anyhow::Result<bool> {
    debug!("Querying sumo-api for basho {} day {}", basho_id.id(), day);
    let resp = BanzukeResponse::fetch(basho_id, RankDivision::Makuuchi).await?;
    let complete = resp.day_complete(day);
    let update_data = resp.torikumi_update_data(day);
    info!(
        "Got day {} results; updating db with {} bouts",
        day,
        update_data.len()
    );

    if *DRY_RUN {
        info!("Dry run; not updating db or sending push notifications");
        for d in &update_data {
            debug!("{} beat {}", d.winner, d.loser);
        }
        return Ok(false);
    }

    update_torikumi(&mut db_conn.lock().unwrap(), basho_id, day, &update_data)?;
    Ok(complete)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WebhookData {
    #[serde(rename = "type")]
    webhook_type: String,
    payload: String, // base64 encoded JSON
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub struct MatchResultsWebhookData {
    date: BashoId,
    start_date: DateTime<Utc>,
    end_date: DateTime<Utc>,
    torikumi: Vec<SumoApiTorikumi>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
struct SumoApiTorikumi {
    id: String,
    basho_id: BashoId,
    division: RankDivision,
    day: u8,
    match_no: u8,
    east_id: u32,
    east_shikona: String,
    east_rank: String,
    west_id: u32,
    west_shikona: String,
    west_rank: String,
    kimarite: String,
    winner_id: u32,
    winner_en: String,
    winner_jp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct RegisterWebhookData {
    name: String,
    destination: String,
    secret: String,
    subscriptions: HashMap<String, bool>,
}

impl RegisterWebhookData {
    fn with_types(config: &Config, types: &[&str]) -> Self {
        let mut subscriptions = HashMap::new();
        for t in types {
            subscriptions.insert(t.to_string(), true);
        }
        let mut destination = config.url();
        destination.set_path("/webhook/sumo_api");
        Self {
            name: format!("kachiclash-{}", config.env),
            destination: destination.to_string(),
            secret: config.webhook_secret.clone(),
            subscriptions,
        }
    }
}

pub async fn register_webhook(config: &Config) -> Result<(), reqwest::Error> {
    let data = RegisterWebhookData::with_types(config, &["matchResults"]);
    info!(
        "Registering webhook with sumo-api; name={} destination={}",
        data.name, data.destination
    );
    make_client()?
        .post("https://www.sumo-api.com/api/webhook/subscribe")
        .json(&data)
        .send()
        .await?;
    Ok(())
}

pub async fn request_webhook_test(
    config: &Config,
    webhook_type: &str,
) -> Result<(), reqwest::Error> {
    let data = RegisterWebhookData::with_types(config, &[webhook_type]);
    let url = format!(
        "https://www.sumo-api.com/api/webhook/test?type={}",
        webhook_type
    );
    info!("Request test webhook for type {:?}", webhook_type);
    make_client()?.post(url).json(&data).send().await?;
    Ok(())
}

fn decode_hex_sha256(s: &str) -> Result<[u8; 32]> {
    if s.len() != 64 {
        bail!("Invalid SHA256 hex length {}", s.len());
    }
    let mut bytes = [0; 32];
    for (i, duo) in s.chars().chunks(2).into_iter().enumerate() {
        bytes[i] = u8::from_str_radix(&duo.collect::<String>(), 16)?;
    }
    Ok(bytes)
}

pub async fn receive_webhook(
    url: &Url,
    body: &[u8],
    sig: &str,
    data: &WebhookData,
    db: &mut Connection,
    secret: &str,
) -> Result<(), anyhow::Error> {
    if *DRY_RUN {
        info!("Receive webhook data (dry run)");
        debug!("sig: {}\nbody: {}", sig, String::from_utf8_lossy(&body));
    }

    let mut hmac = HMAC::new(secret);
    hmac.update(url.as_str());
    hmac.update(body);
    if decode_hex_sha256(sig)? != hmac.finalize() {
        bail!("Invalid signature");
    }

    if data.webhook_type != "matchResults" {
        bail!("Unexpected webhook type {}", data.webhook_type);
    }

    let MatchResultsWebhookData {
        date: basho_id,
        torikumi,
        ..
    } = serde_json::from_str(&data.payload)?;

    let day = torikumi[0].day;
    if torikumi.iter().any(|t| t.day != day) {
        bail!("Mismatched day in torikumi");
    }

    let current_basho_id = BashoInfo::current_or_next_basho_id(db)?;
    if basho_id != current_basho_id {
        bail!(
            "Webhook basho {} is not the current one: {}",
            basho_id,
            current_basho_id
        );
    }

    let update_data = torikumi
        .iter()
        .map(|torikumi| TorikumiMatchUpdateData {
            winner: torikumi.winner_en.clone(),
            loser: if torikumi.winner_en == torikumi.east_shikona {
                torikumi.west_shikona.clone()
            } else {
                torikumi.east_shikona.clone()
            },
        })
        .collect::<Vec<_>>();

    if *DRY_RUN {
        info!("Dry run; not updating db with {} bouts:", update_data.len());
        for d in &update_data {
            debug!("{} beat {}", d.winner, d.loser);
        }
    } else {
        update_torikumi(db, basho_id, day, &update_data)?;
    }
    Ok(())
}

fn make_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(CONNECTION_TIMEOUT))
        .timeout(Duration::from_secs(RESPONSE_TIMEOUT))
        .user_agent("kachiclash.com")
        .build()
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::{BanzukeResponse, BoutResponse, BoutResult};
    use crate::data::{basho::TorikumiMatchUpdateData, BashoId, Rank, RankDivision};

    fn init_logger() {
        let _ = pretty_env_logger::env_logger::builder()
            .is_test(true)
            .try_init();
    }

    const BANZUKE_202307: &str = include_str!("sumo-api-banzuke-202307-makuuchi.json");

    #[tokio::test]
    async fn call_api() {
        init_logger();

        let resp = BanzukeResponse::fetch(202307.into(), RankDivision::Makuuchi)
            .await
            .expect("sumo-api call failed");

        // Spot check a few properties in the response
        assert_eq!(BashoId::from(202307), resp.basho_id);
        assert_eq!(RankDivision::Makuuchi, resp.division);
        assert_eq!(21, resp.east.len());
        assert_eq!(21, resp.west.len());
        let terunofuji = &resp.east[0];
        assert_eq!("Terunofuji", terunofuji.shikona_en);
        assert_eq!("Y1e".parse::<Rank>().unwrap(), terunofuji.rank);
    }

    #[test]
    fn response_parsing() {
        init_logger();

        let resp: BanzukeResponse =
            serde_json::from_str(BANZUKE_202307).expect("parse API response fixture");

        assert_eq!(BashoId::from(202307), resp.basho_id);
        assert_eq!(RankDivision::Makuuchi, resp.division);
        assert_eq!(21, resp.east.len());
        assert_eq!(21, resp.west.len());

        let terunofuji = &resp.east[0];
        assert_eq!("Terunofuji", terunofuji.shikona_en);
        assert_eq!("Y1e".parse::<Rank>().unwrap(), terunofuji.rank);
        assert_eq!(15, terunofuji.record.len());
        assert_eq!(
            BoutResponse {
                result: BoutResult::Win,
                opponent_shikona_en: "Abi".to_string()
            },
            terunofuji.record[0]
        );
        assert_eq!(
            BoutResponse {
                result: BoutResult::Loss,
                opponent_shikona_en: "Nishikigi".to_string()
            },
            terunofuji.record[1]
        );
        assert_eq!(
            BoutResponse {
                result: BoutResult::FusenLoss,
                opponent_shikona_en: "Shodai".to_string()
            },
            terunofuji.record[3]
        );
        assert_eq!(
            BoutResponse {
                result: BoutResult::Absent,
                opponent_shikona_en: "".to_string()
            },
            terunofuji.record[4]
        );

        let takakeisho = &resp.east[1];
        assert_eq!("Takakeisho", takakeisho.shikona_en);
        assert_eq!("O1e".parse::<Rank>().unwrap(), takakeisho.rank);
        assert_eq!(15, takakeisho.record.len());
        assert!(takakeisho
            .record
            .iter()
            .all(|r| r.result == BoutResult::Absent || r.result == BoutResult::None));

        let kirishima = &resp.west[0];
        assert_eq!("Kirishima", kirishima.shikona_en);
        assert_eq!("O1w".parse::<Rank>().unwrap(), kirishima.rank);
        assert_eq!(15, kirishima.record.len());
        assert_eq!(
            BoutResponse {
                result: BoutResult::FusenLoss,
                opponent_shikona_en: "Nishikigi".to_string()
            },
            kirishima.record[0]
        );
        assert_eq!(
            BoutResponse {
                result: BoutResult::Absent,
                opponent_shikona_en: "".to_string()
            },
            kirishima.record[1]
        );
        assert_eq!(
            BoutResponse {
                result: BoutResult::Win,
                opponent_shikona_en: "Kotonowaka".to_string()
            },
            kirishima.record[3]
        );
    }

    #[test]
    fn update_torikumi_data() {
        init_logger();

        let resp: BanzukeResponse =
            serde_json::from_str(BANZUKE_202307).expect("parse API response fixture");
        let mut data = resp.torikumi_update_data(4);
        data.sort_by(|a, b| a.winner.cmp(&b.winner));

        // Takakeisho and Wakatakakage were kyujo on this day resulting in one less than the max possible 21 bouts. Terunofuji had a fusen loss, and Kiriyama came back from kyujo.
        assert_eq!(20, data.len());

        assert_eq!(
            TorikumiMatchUpdateData {
                winner: "Abi".to_owned(),
                loser: "Tobizaru".to_owned(),
            },
            data[0]
        );

        assert!(data
            .iter()
            .flat_map(|d| vec![&d.winner, &d.loser])
            .all_unique());
    }

    #[test]
    fn update_torikumi_data_with_juryo() {
        init_logger();

        let resp: BanzukeResponse =
            serde_json::from_str(BANZUKE_202307).expect("parse API response fixture");
        let mut data = resp.torikumi_update_data(5);
        data.sort_by(|a, b| a.winner.cmp(&b.winner));

        // Terunofuji, Takakeisho, and Wakatakakage were kyujo on this day resulting in one less than the max possible 21 bouts. Bushozan lost against Roga from Juryo, which should be included.
        assert_eq!(20, data.len());
        assert_eq!(
            TorikumiMatchUpdateData {
                winner: "Roga".to_owned(),
                loser: "Bushozan".to_owned(),
            },
            data[13]
        );
    }
}
