use std::time::Duration;

use crate::data::{BashoId, Rank, RankDivision};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BanzukeResponse {
    basho_id: BashoId,
    division: RankDivision,
    east: Vec<RikishiResponse>,
    west: Vec<RikishiResponse>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RikishiResponse {
    shikona_en: String,
    rank: Rank,
    record: Vec<BoutResponse>,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct BoutResponse {
    result: BoutResult,
    opponent_shikona_en: String,
}

#[derive(Debug, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum BoutResult {
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
    pub async fn fetch(
        basho_id: BashoId,
        division: RankDivision,
    ) -> Result<BanzukeResponse, reqwest::Error> {
        let url = format!(
            "https://www.sumo-api.com/api/basho/{basho}/banzuke/{division}",
            basho = basho_id.id()
        );
        debug!("sending request to {}", url);
        make_client()?
            .get(&url)
            .send()
            .await?
            .json::<BanzukeResponse>()
            .await
    }
}

fn make_client() -> reqwest::Result<reqwest::Client> {
    reqwest::Client::builder()
        .connect_timeout(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .user_agent("kachiclash.com")
        .build()
}

#[cfg(test)]
mod tests {
    use super::{BanzukeResponse, BoutResponse, BoutResult};
    use crate::data::{BashoId, Rank, RankDivision};

    fn init_logger() {
        let _ = pretty_env_logger::env_logger::builder()
            .is_test(true)
            .try_init();
    }

    #[tokio::test]
    async fn response_parsing() {
        init_logger();

        let resp = BanzukeResponse::fetch(202307.into(), RankDivision::Makuuchi)
            .await
            .expect("sumo-api call failed");

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
