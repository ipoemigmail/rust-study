use anyhow::Result;
use async_trait::async_trait;
use futures::{future::join_all, stream::{self, StreamExt}};
use log::info;
use std::{rc::Rc, str, sync::Arc, time};

use zookeeper::{WatchedEvent, Watcher, ZooKeeper};
struct LoggingWatcher;
impl Watcher for LoggingWatcher {
    fn handle(&self, e: WatchedEvent) {
        println!("{:?}", e)
    }
}

#[tokio::main]
async fn main() {
    let zk_client = Arc::new(ZooKeeper::connect("localhost:2181", time::Duration::from_secs(1), LoggingWatcher).unwrap());

    //let client = ZooKeeper::connect("localhost:2181", time::Duration::from_secs(1), LoggingWatcher).unwrap();
    let path = "key1";
    let tasks = (0..100000).map(|_| {
        let s = zk_client.clone();
        tokio::task::spawn_blocking(move || { s.get_data("/kakao/commerce/kcdl/v2/jobs/meta/dev-hadoop/local/test.app/config/key1", false) })
    }).collect::<Vec<_>>();
    let a = stream::iter(tasks).then(|f| async move { f.await }).collect::<Vec<_>>().await;
    //let a = join_all(tasks).await;
    println!("{:?}", a[0]);
    //println!("{:?}", data);
}
