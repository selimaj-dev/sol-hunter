pub mod burst;

use std::sync::Arc;

use crate::{
    bot::Bot,
    types::{NewToken, Trade},
};

#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    async fn on_new_coin(&mut self, bot: Arc<Bot>, token: NewToken) -> anyhow::Result<()>;

    async fn on_trade(&mut self, bot: Arc<Bot>, trade: Trade) -> anyhow::Result<()>;
}
