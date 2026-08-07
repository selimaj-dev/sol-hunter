mod pump_fun;

use crate::launchpad::pump_fun::PumpFun;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::{Mutex, mpsc};
use tokio_tungstenite::tungstenite::Message;

pub struct Client(
    pub  Mutex<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
    >,
);

pub struct Executor {
    pub client: Arc<Client>,

    pub pump_fun: Mutex<PumpFun>,
}

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait Launchpad: Send + Sync {
    async fn buy(
        &mut self,
        client: &Client,

        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()>;

    async fn sell(
        &mut self,
        client: &Client,

        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()>;

    fn get_positions(&self) -> HashMap<String, Decimal>;
}

impl Executor {
    pub async fn new(api_key: &str) -> anyhow::Result<Arc<Self>> {
        Ok(Arc::new(Self {
            client: Arc::new(Client(Mutex::new(
                tokio_tungstenite::connect_async(format!(
                    "wss://devnet.helius-rpc.com/?api-key={api_key}"
                ))
                .await?
                .0,
            ))),

            pump_fun: Mutex::new(PumpFun::new()),
        }))
    }

    pub async fn buy(
        self: &Arc<Self>,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        self.pump_fun
            .lock()
            .await
            .buy(&self.client, mint, amount, priority, slippage)
            .await
    }

    pub async fn sell(
        self: &Arc<Self>,
        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        self.pump_fun
            .lock()
            .await
            .sell(&self.client, mint, amount, priority, slippage)
            .await
    }

    pub async fn sell_all(
        self: &Arc<Self>,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        let mut pump = self.pump_fun.lock().await;

        for (mint, _) in pump.get_positions() {
            pump.sell(&self.client, &mint, 100, priority, slippage)
                .await?;
        }

        Ok(())
    }

    pub async fn listen(self: &Arc<Self>) -> anyhow::Result<mpsc::Receiver<u32>> {
        let (tx, rx) = mpsc::channel(10);

        let client = self.client.clone();

        tokio::spawn(async move {
            match client.0.lock().await.next().await {
                Some(Ok(Message::Text(msg))) => {}
                Some(Ok(msg)) => {}
                Some(Err(e)) => {}
                None => {}
            }
        });

        Ok(rx)
    }
}
