mod pump_fun;

use rust_decimal::Decimal;
use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};
use tokio::sync::{Mutex, mpsc};

use crate::{data::Event, launchpad::pump_fun::PumpFun};

type ClientMutex = Mutex<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
>;

pub struct Client {
    pub helius: ClientMutex,
    pub solana: ClientMutex,
    pub subscribed: Mutex<HashSet<String>>,
}

pub struct Executor {
    pub client: Arc<Client>,

    pub event_tx: Mutex<mpsc::Sender<Event>>,

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

    async fn listen(client: Arc<Client>, tx: mpsc::Sender<Event>) -> anyhow::Result<()>;

    fn get_positions(&self) -> HashMap<String, Decimal>;
}

impl Executor {
    pub async fn new(api_key: &str) -> anyhow::Result<Arc<Self>> {
        let (tx, _) = mpsc::channel(1);

        tx.closed().await;

        Ok(Arc::new(Self {
            client: Arc::new(Client {
                helius: Mutex::new(
                    tokio_tungstenite::connect_async(format!(
                        "wss://mainnet.helius-rpc.com/?api-key={api_key}"
                    ))
                    .await?
                    .0,
                ),

                solana: Mutex::new(
                    tokio_tungstenite::connect_async(format!("wss://api.mainnet-beta.solana.com"))
                        .await?
                        .0,
                ),

                subscribed: Mutex::new(HashSet::new()),
            }),

            event_tx: Mutex::new(tx),

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

    pub async fn listen(self: &Arc<Self>) -> anyhow::Result<mpsc::Receiver<Event>> {
        let (tx, rx) = mpsc::channel(100);

        *self.event_tx.lock().await = tx.clone();

        tokio::spawn(PumpFun::listen(self.client.clone(), tx));

        Ok(rx)
    }

    pub async fn subscribe(&self, mint: &str) {
        self.client.subscribed.lock().await.insert(mint.to_string());
    }

    pub async fn unsubscribe(&self, mint: &str) {
        self.client.subscribed.lock().await.remove(mint);
    }
}
