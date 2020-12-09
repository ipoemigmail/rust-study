use actix_web::{guard, web, App, HttpServer, Responder};
use async_graphql::{
    Context, EmptyMutation, EmptySubscription, Enum, MergedObject, Object, Result, Schema,
    SimpleObject, Subscription, ID,
};
use async_graphql_actix_web::{Request, Response};
use futures::{Stream, StreamExt};
use tokio::prelude::*;
use tokio::task::JoinError;
use tokio::task::JoinHandle;
use tokio::time;

#[derive(Clone, SimpleObject)]
pub struct Book {
    id: ID,
    name: String,
    author: String,
}

pub struct BookQuery;

impl Default for BookQuery {
    fn default() -> Self {
        BookQuery
    }
}

#[Object]
impl BookQuery {
    async fn book(&self, ctx: &Context<'_>, id: String) -> Option<Book> {
        Some(Book {
            id: "1".into(),
            name: "test1".to_string(),
            author: "test2".to_string(),
        })
    }

    async fn books(&self, ctx: &Context<'_>) -> Vec<Book> {
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

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let schema = Schema::build(RootQuery::default(), EmptyMutation, EmptySubscription).finish();
    let local = tokio::task::LocalSet::new();
    let sys = actix_rt::System::run_in_tokio("server", &local);
    let server_res = HttpServer::new(move || {
        App::new()
            .data(schema.clone())
            .service(web::resource("/").guard(guard::Post()).to(index))
    })
    .bind("0.0.0.0:8080")?
    .run()
    .await?;
    sys.await?;
    Ok(server_res)
}
