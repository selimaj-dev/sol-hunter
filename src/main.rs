pub mod account;
pub mod bot;
pub mod executor;
pub mod strategy;
pub mod tradelog;
pub mod types;

use tokio::io::{self, AsyncBufReadExt, BufReader};
use tokio::sync::watch;

use crate::bot::Bot;

async fn shutdown_listener(tx: watch::Sender<bool>) {
    let stdin = BufReader::new(io::stdin());
    let mut lines = stdin.lines();

    while let Ok(Some(line)) = lines.next_line().await {
        if line.trim().eq_ignore_ascii_case("exit") {
            println!("Shutdown requested.");
            let _ = tx.send(true);
            break;
        }
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);
    builder.init();

    let (shutdown_tx, shutdown_rx) = watch::channel(false);

    tokio::spawn(shutdown_listener(shutdown_tx));

    let bot = Bot::new().await?;

    bot.start(shutdown_rx).await
}
