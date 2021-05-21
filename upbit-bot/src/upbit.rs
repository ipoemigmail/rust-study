use std::time::Duration;

use async_trait::async_trait;

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Market {
    #[serde(rename = "market")]
    pub market: String,
    #[serde(rename = "korean_name")]
    pub korean_name: String,
    #[serde(rename = "english_name")]
    pub english_name: String,
    #[serde(rename = "market_warning")]
    pub market_warning: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct MarketTicker {
    #[serde(rename = "market")]
    pub market: String,
    #[serde(rename = "trade_date")]
    pub trade_date: String,
    #[serde(rename = "trade_time")]
    pub trade_time: String,
    #[serde(rename = "trade_date_kst")]
    pub trade_date_kst: String,
    #[serde(rename = "trade_time_kst")]
    pub trade_time_kst: String,
    #[serde(rename = "trade_timestamp")]
    pub trade_timestamp: i64,
    #[serde(rename = "opening_price")]
    pub opening_price: f64,
    #[serde(rename = "high_price")]
    pub high_price: f64,
    #[serde(rename = "low_price")]
    pub low_price: f64,
    #[serde(rename = "trade_price")]
    pub trade_price: f64,
    #[serde(rename = "prev_closing_price")]
    pub prev_closing_price: f64,
    #[serde(rename = "change")]
    pub change: String,
    #[serde(rename = "change_price")]
    pub change_price: f64,
    #[serde(rename = "change_rate")]
    pub change_rate: f64,
    #[serde(rename = "signed_change_price")]
    pub signed_change_price: f64,
    #[serde(rename = "signed_change_rate")]
    pub signed_change_rate: f64,
    #[serde(rename = "trade_volume")]
    pub trade_volume: f64,
    #[serde(rename = "acc_trade_price")]
    pub acc_trade_price: f64,
    #[serde(rename = "acc_trade_price_24h")]
    pub acc_trade_price24_h: f64,
    #[serde(rename = "acc_trade_volume")]
    pub acc_trade_volume: f64,
    #[serde(rename = "acc_trade_volume_24h")]
    pub acc_trade_volume24_h: f64,
    #[serde(rename = "highest_52_week_price")]
    pub highest52_week_price: f64,
    #[serde(rename = "highest_52_week_date")]
    pub highest52_week_date: String,
    #[serde(rename = "lowest_52_week_price")]
    pub lowest52_week_price: f64,
    #[serde(rename = "lowest_52_week_date")]
    pub lowest52_week_date: String,
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    ReqwestError(#[from] reqwest::Error),
    #[error("{0}")]
    SerdeError(#[from] serde_json::error::Error),
}

#[async_trait]
pub trait UpbitService: Send + Sync + 'static {
    async fn market_list(&self) -> Result<Vec<Market>, Error>;
    async fn market_ticker_list(&self, market_ids: Vec<String>)
        -> Result<Vec<MarketTicker>, Error>;
}

pub struct UpbitServiceLive {
    client: reqwest::Client,
}

impl UpbitServiceLive {
    pub fn new() -> UpbitServiceLive {
        UpbitServiceLive {
            client: reqwest::ClientBuilder::new()
                .connect_timeout(Duration::from_secs(1))
                .build()
                .unwrap(),
        }
    }
}

const BASE_URL: &str = "https://api.upbit.com/v1";

#[async_trait]
impl UpbitService for UpbitServiceLive {
    async fn market_list(&self) -> Result<Vec<Market>, Error> {
        let url = format!("{}/market/all?isDetails=true", BASE_URL);
        let resp = self
            .client
            .execute(self.client.get(url).build().unwrap())
            .await?;
        let result: Vec<Market> = serde_json::from_str(resp.text().await?.as_str())?;
        Ok(result)
    }

    async fn market_ticker_list(
        &self,
        market_ids: Vec<String>,
    ) -> Result<Vec<MarketTicker>, Error> {
        let url = format!("{}/ticker?markets={}", BASE_URL, market_ids.join(","));
        let resp = self
            .client
            .execute(self.client.get(url).build().unwrap())
            .await?;
        let resp_text = resp.text().await?;
        //println!("{}", resp_text.as_str());
        let result: Vec<MarketTicker> = serde_json::from_str(resp_text.as_str())?;
        Ok(result)
    }
}
