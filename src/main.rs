pub mod account;
pub mod bot;
pub mod executor;
pub mod strategy;
pub mod tradelog;
pub mod types;

use std::sync::Arc;

use chrono::Local;
use tokio::{
    io::{self, AsyncBufReadExt, BufReader},
    sync::watch,
};

use crate::bot::Bot;

async fn shutdown_listener(bot: Arc<Bot>, tx: watch::Sender<bool>) -> anyhow::Result<()> {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        match line.to_lowercase().trim() {
            "exit" | "shutdown" => {
                let _ = tx.send(true);
            }

            "save" => {
                log::info!("Saving trades");

                let now = Local::now();
                let formatted_time = now.format("%m-%d-%H-%M").to_string();

                let filename = format!("sol-hun-{}.csv", formatted_time);

                let mut writer = csv::Writer::from_path(&filename)?;

                for trade in bot.trade_log.lock().await.iter() {
                    writer.serialize(trade)?;
                }

                writer.flush()?;

                log::info!("Saved trades to {}", filename);
            }

            _ => {}
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);
    builder.init();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    let bot = Bot::new().await?;

    tokio::spawn(shutdown_listener(bot.clone(), shutdown_tx));

    bot.start(shutdown_rx).await
}
