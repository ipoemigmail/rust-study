use std::env;

use futures::StreamExt;
use telegram_bot::*;

#[tokio::main]
async fn main() -> Result<(), Error> {
    let token = "MY_TOKEN";
    let api = Api::new(token);

    let req = requests::SendMessage::new(ChannelId::new(-1001331000957), "테스트");
    let result = api.send(req).await;
    println!("{:?}", result);

    Ok(())
}
