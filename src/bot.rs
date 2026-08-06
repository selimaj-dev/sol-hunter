use std::collections::HashMap;

use anyhow::Context;
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use tokio::net::TcpStream;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async};

use crate::{
    account::AccountManager,
    executor::Executor,
    types::{Mode, NewToken, Token, Trade},
};

pub struct Bot {
    pub ws: WebSocketStream<MaybeTlsStream<TcpStream>>,
    pub accounts: AccountManager,
    pub executor: Box<dyn Executor>,
    pub tokens: HashMap<String, Token>,
}

impl Bot {
    pub async fn on_new_coin(&mut self, token: NewToken) -> anyhow::Result<()> {
        self.tokens.insert(
            token.mint.clone(),
            Token {
                mode: Mode::Observing,
                execute_next: false,
            },
        );

        self.subscribe(&token.mint).await?;

        Ok(())
    }

    pub async fn on_trade(&mut self, trade: Trade) -> anyhow::Result<()> {
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

    pub async fn subscribe(&mut self, mint: &str) -> anyhow::Result<()> {
        self.ws
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

    pub async fn unsubscribe(&mut self, mint: &str) -> anyhow::Result<()> {
        self.ws
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
            tokens: HashMap::new(),
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
