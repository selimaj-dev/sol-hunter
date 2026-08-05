pub mod executor;
pub mod launch;

use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio_tungstenite::connect_async;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let (mut ws, _) = connect_async("wss://pumpdev.io/ws").await?;

    ws.send(tokio_tungstenite::tungstenite::Message::Text(
        json!({ "method": "subscribeNewToken" }).to_string().into(),
    ))
    .await?;

    while let Some(msg) = ws.next().await {
        let msg = msg?;
        if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
            let token: launch::PumpDevEvent = serde_json::from_str(&text)?;
            println!("{:?}", token);
        }
    }
    Ok(())
}
