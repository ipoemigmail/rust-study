use futures::Future;
use std::time::Duration;

pub async fn retry<A, E, Fut: Future<Output = Result<A, E>>, F: Fn() -> Fut>(
    cnt: isize,
    pause: Duration,
    f: F,
) -> Result<A, E> {
    let mut c = cnt;
    let mut r = f().await;
    while c > 0 && r.is_err() {
        tokio::time::sleep(pause).await;
        c -= 1;
        r = f().await;
    }
    r
}
