use crate::error::*;
use tokio::runtime::{Builder, Runtime};

pub struct TelegramMessageSender {
    bot: tbot::Bot,
    user: tbot::types::chat::Id,
    rt: Runtime,
}

impl TelegramMessageSender {
    pub fn new(token: &str, user: i64) -> Self {
        let bot = tbot::Bot::new(String::from(token));
        let user = tbot::types::chat::Id(user);
        let rt = Builder::new()
            .enable_all()
            .basic_scheduler()
            .build()
            .unwrap();
        Self { bot, user, rt }
    }

    pub fn send_message(&mut self, message: &str) -> GenericResult<()> {
        self.rt
            .block_on(self.bot.send_message(self.user, message).call())?;
        Ok(())
    }
}
