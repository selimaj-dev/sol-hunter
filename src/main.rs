pub mod account;
pub mod bot;
pub mod executor;
pub mod types;

use crate::bot::Bot;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);
    builder.init();

    let mut bot = Bot::new().await?;

    bot.start().await
}
