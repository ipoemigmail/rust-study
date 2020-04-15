use chrono::Local;
use std::error::Error;
use std::future::Future;
use std::slice::Iter;
use std::time::Duration;
use tokio::prelude::*;
use tokio::task::JoinError;
use tokio::task::JoinHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    //let mut hs: Vec<_> = (0..2).map(|n| tokio::spawn(worker(n))).collect();
    //for h in hs {
    //    h.await;
    //}
    let hs: Vec<_> = (0..1000000).map(|n| tokio::spawn(calc(n))).collect();
    let start_time = Local::now();
    //let rs = join_all(hs).await;
    //let mut rs = vec![];
    //for h in hs {
    //    rs.push(h.await);
    //}
    let rs = join_all(hs).await;
    let end_time = Local::now();
    println!(
        "spend time: {}",
        end_time.timestamp_millis() - start_time.timestamp_millis()
    );
    //rs.iter().for_each(|x| println!("{}", x.as_ref().unwrap()));
    //println!("{:?}", rs);
    Ok(())
}

async fn join_all<'a, I>(xs: I) -> Vec<<I::Item as Future>::Output>
where
    I: IntoIterator,
    I::Item: Future,
{
    let mut rs = vec![];
    for h in xs.into_iter() {
        rs.push(h.await);
    }
    rs
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
