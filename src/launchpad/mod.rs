mod pump_fun;

use crate::launchpad::pump_fun::PumpFun;
use rust_decimal::Decimal;
use std::collections::HashMap;

pub struct Executor {
    pub pump_fun: PumpFun,
}

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait Launchpad: Send + Sync {
    async fn buy(
        &mut self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()>;

    async fn sell(
        &mut self,
        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()>;

    fn get_positions(&self) -> HashMap<String, Decimal>;
}

impl Executor {
    pub fn new() -> Self {
        Self {
            pump_fun: PumpFun::new(),
        }
    }

    pub async fn buy(
        &mut self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        self.pump_fun.buy(mint, amount, priority, slippage).await
    }

    pub async fn sell(
        &mut self,
        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        self.pump_fun.sell(mint, amount, priority, slippage).await
    }

    pub async fn sell_all(&mut self, priority: Decimal, slippage: u16) -> anyhow::Result<()> {
        for (mint, _) in self.pump_fun.get_positions() {
            self.pump_fun.sell(&mint, 100, priority, slippage).await?;
        }

        Ok(())
    }
}
