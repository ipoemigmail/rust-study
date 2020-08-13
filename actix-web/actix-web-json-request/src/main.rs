use actix_web::{web, App, HttpServer, Result};
use serde::Deserialize;

#[derive(Deserialize)]
struct Info {
    username: String,
}

async fn index1() -> Result<String> {
    Ok("Welcome!".to_owned())
}

async fn index2(info: web::Json<Info>) -> Result<String> {
    Ok(format!("Welcome {}!", info.username))
}

#[actix_rt::main]
async fn main() -> std::io::Result<()> {
    HttpServer::new(|| App::new().route("/", web::post().to(index2)).route("/", web::get().to(index1)))
        .bind("127.0.0.1:8088")?
        .run()
        .await
}