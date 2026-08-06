use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::{net::TcpStream, sync::Mutex};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::{
    account::AccountManager,
    executor::Executor,
    strategy::{Strategy, burst::Burst},
};

pub struct Bot {
    pub ws: Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    pub accounts: Mutex<AccountManager>,
    pub executor: Mutex<Box<dyn Executor>>,
    pub strategy: Mutex<Box<dyn Strategy>>,
}

impl Bot {
    pub async fn subscribe(&self, mint: &str) -> anyhow::Result<()> {
        self.ws
            .lock()
            .await
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({
                    "method": "subscribeTokenTrade",
                    "keys": [mint]
                })
                .to_string()
                .into(),
            ))
            .await?;

        Ok(())
    }

    pub async fn unsubscribe(&self, mint: &str) -> anyhow::Result<()> {
        self.ws
            .lock()
            .await
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({
                    "method": "unsubscribeTokenTrade",
                    "keys": [mint]
                })
                .to_string()
                .into(),
            ))
            .await?;

        Ok(())
    }
}

impl Bot {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let accounts = AccountManager::get().await?;

        let account = accounts
            .accounts
            .get(&accounts.active)
            .context("Failed to get account")?
            .clone();

        Ok(Arc::new(Self {
            ws: Mutex::new(connect_async("wss://pumpdev.io/ws").await?.0),
            executor: Mutex::new(Box::new(account.executor())),
            accounts: Mutex::new(accounts),
            strategy: Mutex::new(Box::new(Burst {
                tokens: HashMap::new(),
            })),
        }))
    }

    pub async fn refresh_account(&self) -> anyhow::Result<()> {
        let accounts = self.accounts.lock().await;

        let account = accounts
            .accounts
            .get(&accounts.active)
            .context("Failed to get account")?
            .clone();

        *self.executor.lock().await = Box::new(account.executor());

        Ok(())
    }

    pub async fn initialize_websocket_subscribe(&self) -> anyhow::Result<()> {
        self.ws
            .lock()
            .await
            .send(tokio_tungstenite::tungstenite::Message::Text(
                json!({ "method": "subscribeNewToken" }).to_string().into(),
            ))
            .await?;

        Ok(())
    }

    pub async fn start(self: &Arc<Self>) -> anyhow::Result<()> {
        self.initialize_websocket_subscribe().await?;

        let mut ws = self.ws.lock().await;

        while let Some(msg) = ws.next().await.transpose()? {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                match serde_json::from_str::<crate::types::PumpDevEvent>(&text) {
                    Ok(crate::types::PumpDevEvent::Create(token)) => {
                        drop(ws);
                        self.strategy
                            .lock()
                            .await
                            .on_new_coin(self.clone(), token)
                            .await?;
                        ws = self.ws.lock().await;
                    }

                    Ok(crate::types::PumpDevEvent::Trade(trade)) => {
                        drop(ws);
                        self.strategy
                            .lock()
                            .await
                            .on_trade(self.clone(), trade)
                            .await?;
                        ws = self.ws.lock().await;
                    }

                    Ok(event) => {
                        println!("{:?}", event);
                    }

                    Err(err) => {
                        log::error!("{err}, MSG -> {text}");
                    }
                }
            }
        }

        Ok(())
    }
}
