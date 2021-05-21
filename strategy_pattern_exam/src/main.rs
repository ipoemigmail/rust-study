use std::sync::Arc;

use async_trait::async_trait;

#[async_trait]
pub trait AService: Send + Sync + 'static {
    async fn a_method(&self);
}

#[async_trait]
pub trait BService: Send + Sync + 'static {
    async fn b_method(&self);
}

pub struct AServiceLive;

#[async_trait]
impl AService for AServiceLive {
    async fn a_method(&self) {
        //println!("a")
    }
}

pub struct BServiceLive {
    pub a_service: Arc<dyn AService>,
}

#[async_trait]
impl BService for BServiceLive {
    async fn b_method(&self) {
        self.a_service.a_method().await;
        //println!("b")
    }
}

#[tokio::main]
async fn main() {
    let a_service = Arc::new(AServiceLive);
    let b_service = Arc::new(BServiceLive { a_service });
    for _ in 0..10000000 {
        b_service.b_method().await;
    }
}
