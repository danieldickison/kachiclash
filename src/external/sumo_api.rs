use std::{collections::HashSet, time::Duration};

use crate::data::{basho::TorikumiMatchUpdateData, BashoId, Rank, RankDivision};

const CONNECTION_TIMEOUT: u64 = 10;
const RESPONSE_TIMEOUT: u64 = 20;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BanzukeResponse {
    pub basho_id: BashoId,
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
        self.all_rikishi()
            .all(|rikishi| rikishi.record[day_idx].result != BoutResult::None)
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

// {
//   "bashoId": "202307",
//   "division": "Makuuchi",
//   "east": [
//     {
//       "side": "East",
//       "rikishiID": 45,
//       "shikonaEn": "Terunofuji",
//       "rank": "Yokozuna 1 East",
//       "record": [
//         {
//           "result": "win",
//           "opponentShikonaEn": "Abi",
//           "opponentShikonaJp": "é˜¿ç‚Ž",
//           "opponentID": 22,
//           "kimarite": "oshidashi"
//         },
