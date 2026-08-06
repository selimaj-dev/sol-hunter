pub mod executor;
pub mod types;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut builder = env_logger::Builder::from_default_env();
    builder.filter_level(log::LevelFilter::Info);
    builder.init();

    let (mut ws, _) = connect_async("wss://pumpdev.io/ws").await?;

    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({ "method": "subscribeNewToken" }).to_string().into(),
    ))
    .await?;

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            match serde_json::from_str::<types::PumpDevEvent>(&text) {
                Ok(event) => {
                    println!("{:?}", event);
                }

                Err(err) => {
                    log::error!("{err}");
                }
            }
        }
    }
    Ok(())
}
