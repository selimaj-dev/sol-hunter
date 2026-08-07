use std::{collections::HashMap, sync::Arc};

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::{
    net::TcpStream,
    sync::{Mutex, watch},
};
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::{
    account::AccountManager,
    executor::ExecutorWrapper,
    strategy::{Strategy, veloc::MomentumVelocityStrategy},
    tradelog::TradeLog,
};

pub struct Bot {
    pub ws: Mutex<WebSocketStream<MaybeTlsStream<TcpStream>>>,
    pub accounts: Mutex<AccountManager>,
    pub executor: Mutex<ExecutorWrapper>,
    pub strategy: Mutex<Box<dyn Strategy>>,
    pub trade_log: Mutex<Vec<TradeLog>>,
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
            executor: Mutex::new(ExecutorWrapper {
                executor: Box::new(account.executor()),
                positions: HashMap::new(),
            }),
            accounts: Mutex::new(accounts),
            strategy: Mutex::new(Box::new(MomentumVelocityStrategy::new())),
            trade_log: Mutex::new(Vec::new()),
        }))
    }

    pub async fn refresh_account(self: &Arc<Self>) -> anyhow::Result<()> {
        self.strategy
            .lock()
            .await
            .execute_sell_all(self.clone())
            .await?;

        let accounts = self.accounts.lock().await;

        let account = accounts
            .accounts
            .get(&accounts.active)
            .context("Failed to get account")?
            .clone();

        self.executor.lock().await.executor = Box::new(account.executor());

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

    pub async fn start(
        self: &Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        self.initialize_websocket_subscribe().await?;

        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        log::info!("Shutdown signal received.");

                        self.strategy
                            .lock()
                            .await
                            .execute_sell_all(self.clone())
                            .await?;

                        log::info!("Bye!");

                        break;
                    }
                }

                result = self.tick() => {
                    if result? {
                        log::warn!("Websocket closed.");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn tick(self: &Arc<Self>) -> anyhow::Result<bool> {
        let mut ws = self.ws.lock().await;

        if let Some(msg) = ws.next().await.transpose()? {
            if let tokio_tungstenite::tungstenite::Message::Text(text) = msg {
                match serde_json::from_str::<crate::types::PumpDevEvent>(&text) {
                    Ok(crate::types::PumpDevEvent::Create(token)) => {
                        drop(ws);

                        self.strategy
                            .lock()
                            .await
                            .on_new_coin(self.clone(), token)
                            .await?;
                    }

                    Ok(crate::types::PumpDevEvent::Trade(trade)) => {
                        drop(ws);

                        self.strategy
                            .lock()
                            .await
                            .on_trade(self.clone(), trade)
                            .await?;
                    }

                    Ok(_event) => {
                        // println!("{:?}", event);
                    }

                    Err(err) => {
                        log::error!("{err}, MSG -> {text}");
                    }
                }
            }

            Ok(false)
        } else {
            Ok(true)
        }
    }
}
