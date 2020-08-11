use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug)]
struct Record {
    pub ppkey: i64,
    pub keyword: String,
    pub image_url: String,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let url = "http://facerecog.devel.kakao.com/v2/list";
    let resp = reqwest::get(url).await?;
    let m: serde_json::Value = serde_json::from_str(resp.text().await?.as_str())?;
    read_record(m)
        .unwrap_or(vec![])
        .iter()
        .for_each(|v| println!("{:?}", v));
    Ok(())
}

fn show_first(v: Value) {
    v.as_object()
        .unwrap()
        .get("serviced")
        .unwrap()
        .as_array()
        .unwrap()
        .iter()
        .take(1)
        .for_each(|v| println!("{:?}", v))
}

fn read_record(v: Value) -> Option<Vec<Record>> {
    v.as_object()?
        .get("serviced")?
        .as_array()?
        .iter()
        .take(100)
        .map(|x| {
            let a = x.as_object()?;
            Some(Record {
                ppkey: a.get("ppkey")?.as_i64()?,
                keyword: a.get("keyword")?.as_str()?.to_owned(),
                image_url: a.get("image_url")?.as_str()?.to_owned(),
            })
        })
        .collect()
}
