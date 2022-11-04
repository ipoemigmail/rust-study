use anyhow::Result;
use async_lock::RwLock;
use axum::{
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Extension, Router,
};
use axum_server::{self, tls_rustls::RustlsConfig};
use clap::Parser;
use cookie::SameSite;
use std::{collections::HashMap, net::SocketAddr, sync::Arc};
use tower_cookies::{Cookie, CookieManagerLayer, Cookies};

#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Args {
    #[clap(short, long)]
    hostname: String,
    #[clap(short, long)]
    port: u16,
}

static COOKIE_NAME: &str = "jsession";

#[derive(Default)]
struct State {
    db: HashMap<String, String>,
}

type SharedState = Arc<RwLock<State>>;

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let now = chrono::Local::now();

    let shared_state = SharedState::default();
    let hostname = args.hostname;
    let addr = SocketAddr::from(([0, 0, 0, 0], args.port));
    println!("[{}] hostname: {}", now, &hostname);
    println!("[{}] port: {}", now, args.port);
    {
        let mut state = shared_state.write().await;
        state.db.insert("hostname".to_string(), hostname.clone());
        state.db.insert("port".to_owned(), format!("{}", args.port));
    }
    let config = RustlsConfig::from_pem_file(
        format!(
            "{}.crt",
            &hostname
        ),
        format!(
            "{}.key",
            &hostname
        ),
    )
    .await
    .unwrap();

    let app = Router::new()
        .route("/a", get(handler))
        .route("/b", get(show_form).merge(post(show_form)))
        .route("/c", get(handler2))
        .layer(CookieManagerLayer::new())
        .layer(Extension(shared_state));

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn handler(cookies: Cookies) -> &'static str {
    let session_cookie = Cookie::build(COOKIE_NAME, "1")
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .finish();
    cookies.add(session_cookie);
    "Check your cookies."
}

async fn show_form(Extension(shared_state): Extension<SharedState>, cookies: Cookies) -> Response {
    let value = match cookies.get(COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => "".to_string(),
    };
    let state = shared_state.read().await;
    let hostname = state.db.get("hostname").cloned().unwrap();
    let port = state
        .db
        .get("port")
        .cloned()
        .unwrap()
        .parse::<u16>()
        .unwrap();

    let other_hostname = if hostname == "host1" {
        "host2"
    } else {
        "host1"
    };
    let other_port = if port == 3000 { 3001 } else { 3000 };
    let contents = format!(
        r#"
        <!doctype html>
        <html>
            <head>
            </head>
            <body>
                jsessionid: {}
                <form action="https://{}:{}/b" method="post">
                    <input type="submit" value="Subscribe!">
                </form>
            </body>
        </html>
        "#,
        value, other_hostname, other_port
    );
    Html(contents).into_response()
}

async fn handler2(cookies: Cookies) -> String {
    let value = match cookies.get(COOKIE_NAME) {
        Some(cookie) => cookie.value().to_string(),
        None => "".to_string(),
    };
    format!("jsession: {}", value)
}
