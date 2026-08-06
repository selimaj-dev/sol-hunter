use std::{collections::HashMap, sync::Arc};

use crate::{
    bot::Bot,
    strategy::Strategy,
    types::{Mode, NewToken, Token, Trade},
};

pub struct Strat {
    pub tokens: HashMap<String, Token>,
}

#[async_trait::async_trait]
impl Strategy for Strat {
    async fn on_new_coin(&mut self, bot: Arc<Bot>, token: NewToken) -> anyhow::Result<()> {
        self.tokens.insert(
            token.mint.clone(),
            Token {
                mode: Mode::Observing,
                execute_next: false,
            },
        );

        bot.subscribe(&token.mint).await?;

        Ok(())
    }

    async fn on_trade(&mut self, bot: Arc<Bot>, trade: Trade) -> anyhow::Result<()> {
        log::info!("Trade: {trade:?}");

        let Some(token) = self.tokens.get_mut(&trade.mint) else {
            log::error!("Token not found on trade: {:?}", trade.mint);
            return Ok(());
        };

        match &token.mode {
            Mode::Observing => {}

            Mode::WaitingForEntry => {}

            Mode::WaitingForExit => {}
        }

        Ok(())
    }
}
