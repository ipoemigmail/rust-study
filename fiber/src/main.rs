use std::error::Error;

use std::time::Duration;

use chrono::Local;
use futures::stream::{self, StreamExt};
use itertools::*;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let start_time = Local::now();
    let fibers = (0..1000000)
        .into_iter()
        .map(|n| tokio::spawn(calc(n)))
        .collect_vec();
    let _results: Vec<_> = stream::iter(fibers)
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
    tokio::time::sleep(Duration::from_secs(1)).await;
    n
}
