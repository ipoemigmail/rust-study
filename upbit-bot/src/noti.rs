use async_trait::async_trait;
use teloxide::{RequestError, prelude::*};

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("{0}")]
    RequestError(#[from] RequestError),
    #[error("{0}")]
    InternalError(String),
}

#[async_trait]
pub trait NotiSender: Send + Sync {
    async fn send_msg(&self, msg: &str) -> Result<(), Error>;
}

#[derive(Clone, Debug)]
pub struct NotiSenderTelegram {
    token: String,
    chat_id: i64,
}

impl NotiSenderTelegram {
    pub fn new(token: String, chat_id: i64) -> NotiSenderTelegram {
        NotiSenderTelegram { token, chat_id }
    }
}

#[async_trait]
impl NotiSender for NotiSenderTelegram {
    async fn send_msg(&self, msg: &str) -> Result<(), Error> {
        //let bot = Bot::new(self.token.clone());
        //bot.send_message(self.chat_id, msg).send().await?;
        Ok(())
    }
}
