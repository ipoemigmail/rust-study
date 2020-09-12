use std::error::Error;
use std::future::Future;
use std::slice::Iter;
use std::time::Duration;

use chrono::Local;
use futures::stream::{self, StreamExt};
use tokio::prelude::*;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio::time;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let start_time = Local::now();
    let fibers: Vec<_> = (0..10000000)
        .into_iter()
        .map(|n| tokio::spawn(calc(n)))
        .collect();
    //let fibers: Vec<_> = (0..10000000).into_iter().map(|n| tokio::spawn(async move {
    //  time::delay_for(Duration::from_secs(1)).await;
    //  n
    //})).collect();
    let results: Vec<_> = stream::iter(fibers)
        .then(|f| async move { f.await })
        .collect()
        .await;
    let end_time = Local::now();
    println!(
        "spend time: {}",
        end_time.timestamp_millis() - start_time.timestamp_millis()
    );
    println!("Done");
    Ok(())
}

async fn calc(n: i32) -> i32 {
    tokio::time::delay_for(Duration::from_secs(1)).await;
    n
}
