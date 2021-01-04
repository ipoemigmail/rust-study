use actix_web::{guard, web, App, HttpServer};
use async_graphql::{
    Context, EmptyMutation, EmptySubscription, MergedObject, Object, Schema, SimpleObject, ID,
};
use async_graphql_actix_web::{Request, Response};
use typed_builder::TypedBuilder;

mod service;

#[derive(Clone, SimpleObject, TypedBuilder)]
pub struct Book {
    pub id: ID,
    pub name: String,
    pub author: String,
}

pub struct BookQuery;

impl Default for BookQuery {
    fn default() -> Self {
        BookQuery
    }
}

#[Object]
impl BookQuery {
    async fn book(&self, _ctx: &Context<'_>, id: String) -> Option<Book> {
        log::debug!("call book id: {}", id);
        Some(
            Book::builder()
                .id(id.clone().into())
                .name("test1".into())
                .author("test2".into())
                .build(),
        )
    }

    async fn books(&self, _ctx: &Context<'_>) -> Vec<Book> {
        log::debug!("call books");
        vec![Book {
            id: "1".into(),
            name: "test1".to_string(),
            author: "test2".to_string(),
        }]
    }
}

#[derive(MergedObject, Default)]
struct RootQuery(BookQuery);

type RootSchema = Schema<RootQuery, EmptyMutation, EmptySubscription>;

async fn index(schema: web::Data<RootSchema>, req: Request) -> Response {
    schema.execute(req.into_inner()).await.into()
}

use service::hello_there::HelloThereService;
use service::{
    hello::{HelloService, HelloServiecDefault},
    hello_there::HelloThereServiceDefault,
};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    std::env::set_var("RUST_LOG", "debug");
    env_logger::init();

    let hello_service = HelloServiecDefault;
    let hello_there_service = HelloThereServiceDefault::builder()
        .hello_service(hello_service)
        .build();
    println!("{}", hello_there_service.hello_there("a"));

    let host = std::env::var("HOST").unwrap_or("0.0.0.0".to_string());
    let port = std::env::var("PORT")
        .ok()
        .and_then(|x| x.parse::<i32>().ok())
        .unwrap_or(8080);
    let schema = Schema::build(RootQuery::default(), EmptyMutation, EmptySubscription).finish();
    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);
    let server = HttpServer::new(move || {
        App::new()
            .data(schema.clone())
            .service(web::resource("/").guard(guard::Post()).to(index))
    })
    .bind(format!("{}:{}", host, port))?
    .run();
    println!("server listen {}:{}", host, port);
    let server_res = server.await?;
    sys.await?;
    Ok(server_res)
}
