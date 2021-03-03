mod upbit_service;

use std::time::Duration;
use tokio::time;
use upbit_service::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut interval = time::interval(Duration::from_secs(1));
    loop {
        interval.tick().await;
        let s = UpbitServiceLive.market_list().await?;
        //s.iter().for_each(|x| println!("{:?}", x));
        let ids = s
            .into_iter()
            .filter(|x| x.market.starts_with("KRW"))
            .map(|x| x.market)
            .collect::<Vec<_>>();
        let m = UpbitServiceLive.market_ticker_list(ids).await?;
        //m.iter().for_each(|x| println!("{:?}", x));
        println!("{}", m.len());
    }
}
