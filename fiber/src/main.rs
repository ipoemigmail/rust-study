use std::error::Error;
use std::time::Duration;

use std::future::Future;
use tokio::prelude::*;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let mut hs: Vec<_> = (0..2).map(|n| tokio::spawn(worker(n))).collect();
    //for h in hs {
    //    h.await;
    //}
    let mut hs: Vec<_> = (0..1000000).map(|n| tokio::spawn(calc(n))).collect();
    let mut rs = vec![];
    for h in hs {
        rs.push(h.await.unwrap());
    }
    //println!("{:?}", rs);
    Ok(())
}

async fn worker(n: i32) -> () {
    let mut cnt: i64 = 0;
    loop {
        println!("worker {} running {}", n, cnt);
        cnt += 1;
        tokio::time::delay_for(Duration::from_secs(1)).await;
    }
}

async fn calc(n: i32) -> i32 {
    tokio::time::delay_for(Duration::from_secs(1)).await;
    n
}
