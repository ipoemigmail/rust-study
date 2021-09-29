use format_num::NumberFormat;
use rust_decimal::prelude::*;
use std::sync::Arc;

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct MarketInfo {
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
pub struct Ticker {
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
    pub opening_price: Decimal,
    #[serde(rename = "high_price")]
    pub high_price: Decimal,
    #[serde(rename = "low_price")]
    pub low_price: Decimal,
    #[serde(rename = "trade_price")]
    pub trade_price: Decimal,
    #[serde(rename = "prev_closing_price")]
    pub prev_closing_price: Decimal,
    #[serde(rename = "change")]
    pub change: String,
    #[serde(rename = "change_price")]
    pub change_price: Decimal,
    #[serde(rename = "change_rate")]
    pub change_rate: f64,
    #[serde(rename = "signed_change_price")]
    pub signed_change_price: Decimal,
    #[serde(rename = "signed_change_rate")]
    pub signed_change_rate: f64,
    #[serde(rename = "trade_volume")]
    pub trade_volume: Decimal,
    #[serde(rename = "acc_trade_price")]
    pub acc_trade_price: Decimal,
    #[serde(rename = "acc_trade_price_24h")]
    pub acc_trade_price24_h: Decimal,
    #[serde(rename = "acc_trade_volume")]
    pub acc_trade_volume: Decimal,
    #[serde(rename = "acc_trade_volume_24h")]
    pub acc_trade_volume24_h: Decimal,
    #[serde(rename = "highest_52_week_price")]
    pub highest52_week_price: Decimal,
    #[serde(rename = "highest_52_week_date")]
    pub highest52_week_date: String,
    #[serde(rename = "lowest_52_week_price")]
    pub lowest52_week_price: Decimal,
    #[serde(rename = "lowest_52_week_date")]
    pub lowest52_week_date: String,
    #[serde(rename = "timestamp")]
    pub timestamp: i64,
}

#[derive(
    Default, Debug, Clone, PartialEq, Eq, Hash, serde_derive::Serialize, serde_derive::Deserialize,
)]
pub struct Account {
    #[serde(rename = "currency")]
    pub currency: String,
    #[serde(rename = "balance")]
    pub balance: Decimal,
    #[serde(rename = "locked")]
    pub locked: Decimal,
    #[serde(rename = "avg_buy_price")]
    pub avg_buy_price: Decimal,
    #[serde(rename = "avg_buy_price_modified")]
    pub avg_buy_price_modified: bool,
    #[serde(rename = "unit_currency")]
    pub unit_currency: String,
}

impl std::fmt::Display for Account {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let num = NumberFormat::new();
        let fmtstr = ",.5";
        let balance = num.format(fmtstr, self.balance.to_f64().unwrap());
        let avg_buy_price = num.format(fmtstr, self.avg_buy_price.to_f64().unwrap());
        f.write_fmt(format_args!(
            "{}: Balance - {}, Locked - {}, AverageBuyPrice - {}, UnitCurrency -  {}",
            self.currency, balance, self.locked, avg_buy_price, self.unit_currency
        ))
    }
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct Candle {
    pub market: String,
    #[serde(rename = "candle_date_time_utc")]
    pub candle_date_time_utc: String,
    #[serde(rename = "candle_date_time_kst")]
    pub candle_date_time_kst: String,
    #[serde(rename = "opening_price")]
    pub opening_price: Decimal,
    #[serde(rename = "high_price")]
    pub high_price: Decimal,
    #[serde(rename = "low_price")]
    pub low_price: Decimal,
    #[serde(rename = "trade_price")]
    pub trade_price: Decimal,
    pub timestamp: i64,
    #[serde(rename = "candle_acc_trade_price")]
    pub candle_acc_trade_price: Decimal,
    #[serde(rename = "candle_acc_trade_volume")]
    pub candle_acc_trade_volume: Decimal,
    pub unit: u8,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderChance {
    #[serde(rename = "bid_fee")]
    pub bid_fee: Decimal,
    #[serde(rename = "ask_fee")]
    pub ask_fee: Decimal,
    #[serde(rename = "market")]
    pub market: OrderMarket,
    #[serde(rename = "bid_account")]
    pub bid_account: Account,
    #[serde(rename = "ask_account")]
    pub ask_account: Account,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderMarket {
    #[serde(rename = "id")]
    pub id: String,
    #[serde(rename = "name")]
    pub name: String,
    #[serde(rename = "order_types")]
    pub order_types: Arc<Vec<String>>,
    #[serde(rename = "order_sides")]
    pub order_sides: Arc<Vec<String>>,
    #[serde(rename = "bid")]
    pub bid: OrderMin,
    #[serde(rename = "ask")]
    pub ask: OrderMin,
    #[serde(rename = "max_total")]
    pub max_total: Decimal,
    #[serde(rename = "state")]
    pub state: String,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderMin {
    #[serde(rename = "currency")]
    pub currency: String,
    #[serde(rename = "price_unit")]
    pub price_unit: Option<String>,
    #[serde(rename = "min_total")]
    pub min_total: Decimal,
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct TickerWs {
    #[serde(rename = "type")]
    pub type_field: String,
    pub code: String,
    #[serde(rename = "opening_price")]
    pub opening_price: Decimal,
    #[serde(rename = "high_price")]
    pub high_price: Decimal,
    #[serde(rename = "low_price")]
    pub low_price: Decimal,
    #[serde(rename = "trade_price")]
    pub trade_price: Decimal,
    #[serde(rename = "prev_closing_price")]
    pub prev_closing_price: Decimal,
    #[serde(rename = "acc_trade_price")]
    pub acc_trade_price: Decimal,
    pub change: String,
    #[serde(rename = "change_price")]
    pub change_price: Decimal,
    #[serde(rename = "signed_change_price")]
    pub signed_change_price: Decimal,
    #[serde(rename = "change_rate")]
    pub change_rate: f64,
    #[serde(rename = "signed_change_rate")]
    pub signed_change_rate: f64,
    #[serde(rename = "ask_bid")]
    pub ask_bid: String,
    #[serde(rename = "trade_volume")]
    pub trade_volume: Decimal,
    #[serde(rename = "acc_trade_volume")]
    pub acc_trade_volume: Decimal,
    #[serde(rename = "trade_date")]
    pub trade_date: String,
    #[serde(rename = "trade_time")]
    pub trade_time: String,
    #[serde(rename = "trade_timestamp")]
    pub trade_timestamp: i64,
    #[serde(rename = "acc_ask_volume")]
    pub acc_ask_volume: Decimal,
    #[serde(rename = "acc_bid_volume")]
    pub acc_bid_volume: Decimal,
    #[serde(rename = "highest_52_week_price")]
    pub highest52_week_price: Decimal,
    #[serde(rename = "highest_52_week_date")]
    pub highest52_week_date: String,
    #[serde(rename = "lowest_52_week_price")]
    pub lowest52_week_price: Decimal,
    #[serde(rename = "lowest_52_week_date")]
    pub lowest52_week_date: String,
    #[serde(rename = "trade_status")]
    pub trade_status: Option<String>,
    #[serde(rename = "market_state")]
    pub market_state: String,
    #[serde(rename = "market_state_for_ios")]
    pub market_state_for_ios: Option<String>,
    #[serde(rename = "is_trading_suspended")]
    pub is_trading_suspended: bool,
    #[serde(rename = "delisting_date")]
    pub delisting_date: Option<String>,
    #[serde(rename = "market_warning")]
    pub market_warning: String,
    pub timestamp: i64,
    #[serde(rename = "acc_trade_price_24h")]
    pub acc_trade_price24_h: Decimal,
    #[serde(rename = "acc_trade_volume_24h")]
    pub acc_trade_volume24_h: Decimal,
    #[serde(rename = "stream_type")]
    pub stream_type: String,
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct RemainReq {
    pub group: String,
    pub min: u32,
    pub max: u32,
}

impl RemainReq {
    pub fn new(group: &str, min: u32, max: u32) -> RemainReq {
        RemainReq {
            group: group.to_owned(),
            min,
            max,
        }
    }
}

#[derive(Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub enum OrderSide {
    #[serde(rename = "bid")]
    Bid, // 매수
    #[serde(rename = "ask")]
    Ask, // 매도
}

impl Default for OrderSide {
    fn default() -> Self {
        OrderSide::Ask
    }
}

#[derive(Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub enum OrderType {
    #[serde(rename = "limit")]
    Limit, // 지정가
    #[serde(rename = "price")]
    Price, // 시장가 (매수)
    #[serde(rename = "market")]
    Market, // 시장가 (매도)
}

impl Default for OrderType {
    fn default() -> Self {
        OrderType::Limit
    }
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderRequest {
    pub market: String,
    pub side: OrderSide,
    pub volume: Decimal,
    pub price: Decimal,
    pub order_type: OrderType,
    pub identifier: Option<String>,
}

impl OrderRequest {
    pub fn new(
        market: String,
        side: OrderSide,
        volume: Decimal,
        price: Decimal,
        order_type: OrderType,
        identifier: Option<String>,
    ) -> OrderRequest {
        OrderRequest {
            market,
            side,
            volume,
            price,
            order_type,
            identifier,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, serde_derive::Serialize, serde_derive::Deserialize)]
pub struct OrderResponse {
    pub uuid: String,
    pub side: String,
    #[serde(rename = "ord_type")]
    pub ord_type: String,
    pub price: Decimal,
    #[serde(rename = "avg_price")]
    pub avg_price: Decimal,
    pub state: String,
    pub market: String,
    #[serde(rename = "created_at")]
    pub created_at: String,
    pub volume: Decimal,
    #[serde(rename = "remaining_volume")]
    pub remaining_volume: Decimal,
    #[serde(rename = "reserved_fee")]
    pub reserved_fee: Decimal,
    #[serde(rename = "remaining_fee")]
    pub remaining_fee: Decimal,
    #[serde(rename = "paid_fee")]
    pub paid_fee: Decimal,
    pub locked: Decimal,
    #[serde(rename = "executed_volume")]
    pub executed_volume: Decimal,
    #[serde(rename = "trades_count")]
    pub trades_count: i64,
}
