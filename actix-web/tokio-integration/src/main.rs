use tokio::prelude::*;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio::time;

use actix_web::{Responder, HttpServer, App, web};

async fn hello_world() -> impl Responder {
    let task = tokio::spawn(async move { "Hello World!" });
    task.await.unwrap()
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);
    let server_res = HttpServer::new(|| App::new().route("/", web::get().to(hello_world)))
        .bind("0.0.0.0:8000")?
        .run()
        .await?;
    sys.await?;
    Ok(server_res)
}
