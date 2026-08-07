mod pump_fun;

use crate::launchpad::pump_fun::PumpFun;
use helius::Helius;
use rust_decimal::Decimal;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::Mutex;

pub struct Executor {
    pub client: Helius,

    pub pump_fun: Mutex<PumpFun>,
}

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait Launchpad: Send + Sync {
    async fn buy(
        &mut self,
        client: &Helius,

        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()>;

    async fn sell(
        &mut self,
        client: &Helius,

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
            client: Helius::new_async(api_key, helius::types::Cluster::Devnet).await?,

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
}
