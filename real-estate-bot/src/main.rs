use futures::TryFutureExt;
use futures::{
    stream::{self, StreamExt},
    FutureExt,
};

use estate::EstateServiceLive;

use crate::estate::{Complex, ComplexArticle, Error, EstateService};
use std::{convert::identity, rc::Rc};
use std::sync::Arc;

mod estate;

const GANG_NAM_GU: &'static str = "1168000000";
const SONG_PA_GU: &'static str = "1171000000";
const SU_JUNG_GU: &'static str = "4113100000";

const CHANG_GOK_DONG: &'static str = "4113110800";

#[tokio::main]
async fn main() {
}

fn result_flatten<T, E: From<E1>, E1>(r: Result<Result<T, E>, E1>) -> Result<T, E> {
    r.map_err(|e| e.into()).and_then(|y| y)
}

async fn get_result_list<T: EstateService>(
    service: Arc<T>,
    region_no: String,
) -> Result<Vec<ComplexArticle>, estate::Error> {
    let xs = service.clone().complex_list(region_no).await?;
    stream::iter(xs)
        .map(|x| service.clone().complex_article_list(x.cortar_no))
        .then(tokio::spawn)
        .collect::<Vec<_>>()
        .await
        .into_iter()
        .map(result_flatten)
        .collect::<Result<Vec<_>, _>>()
        .map(|xs| xs.into_iter().flatten().collect::<Vec<_>>())
}
