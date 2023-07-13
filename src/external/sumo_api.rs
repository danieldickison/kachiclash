use std::time::Duration;

use serde::de::DeserializeOwned;

use crate::data::{BashoId, RankDivision};

pub async fn fetch_torikumi(basho_id: BashoId, day: u8) {}

enum Endpoint {
    Banzuke(BashoId, RankDivision),
    Torikumi(BashoId, RankDivision, u8),
}

impl Endpoint {
    pub async fn fetch(&self) -> reqwest::Result<impl DeserializeOwned> {
        let url = self.url();
        debug!("sending request to {}", url);
        Self::make_client()?
            .get(&url)
            .send()
            .await?
            .json::<T>() // TODO: can enum variants have separate associated types??
            .await
    }

    fn url(&self) -> String {
        let basho = match self {
            Endpoint::Banzuke(basho, _) | Endpoint::Torikumi(basho, _, _) => basho,
        };
        let subject = match self {
            Endpoint::Banzuke(_, _) => "banzuke",
            Endpoint::Torikumi(_, _, _) => "torikumi",
        };
        let division = match self {
            Endpoint::Banzuke(_, div) | Endpoint::Torikumi(_, div, _) => div,
        };
        let day_suffix = match self {
            Endpoint::Banzuke(_, _) => "",
            Endpoint::Torikumi(_, _, day) => &format!("/{}", day),
        };
        format!("https://www.sumo-api.com/api/basho/{basho}/{subject}/{division}{day_suffix}")
    }

    fn make_client() -> reqwest::Result<reqwest::Client> {
        reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(5))
            .timeout(Duration::from_secs(10))
            .user_agent("kachiclash.com")
            .build()
    }
}
