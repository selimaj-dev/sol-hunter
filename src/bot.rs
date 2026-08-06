use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::{account::AccountManager, executor::Executor, types::NewToken};

pub struct Bot {
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pub accounts: AccountManager,
    pub executor: Box<dyn Executor>,
}

impl Bot {
    pub async fn on_new_coin(&mut self, token: NewToken) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Bot {
    pub async fn new() -> anyhow::Result<Self> {
        let accounts = AccountManager::get().await?;

        let account = accounts
            .accounts
            .get(&accounts.active)
            .context("Failed to get account")?
            .clone();

        Ok(Self {
            ws: connect_async("wss://pumpdev.io/ws").await?.0,
            executor: Box::new(account.executor()),
            accounts,
        })
    }

    pub async fn refresh_account(&mut self) -> anyhow::Result<()> {
        let account = self
            .accounts
            .accounts
            .get(&self.accounts.active)
            .context("Failed to get account")?
            .clone();

        self.executor = Box::new(account.executor());

        Ok(())
    }

    pub async fn initialize_websocket_subscribe(&mut self) -> anyhow::Result<()> {
        self.ws
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({ "method": "subscribeNewToken" }).to_string().into(),
            ))
            .await?;

        Ok(())
    }

    pub async fn start(&mut self) -> anyhow::Result<()> {
        self.initialize_websocket_subscribe().await?;

        while let Some(msg) = self.ws.next().await {
            let msg = msg?;
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                match serde_json::from_str::<crate::types::PumpDevEvent>(&text) {
                    Ok(crate::types::PumpDevEvent::Create(token)) => {
                        self.on_new_coin(token).await?;
                    }

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
}
