pub mod pump_fun;

use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::account::Account;

pub struct ExecutorWrapper {
    pub executor: Box<dyn Executor>,
    pub positions: HashMap<String, Decimal>,
}

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait Executor: Send + Sync {
    async fn buy(
        &self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sell(
        &self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sell_percent(
        &self,
        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}

impl Account {
    pub fn executor(self) -> impl Executor {
        match self {
            Self::PumpDev(account) => pump_fun::PumpDev::new(account),
        }
    }
}

impl ExecutorWrapper {
    pub async fn buy(
        &mut self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        self.executor.buy(mint, amount, priority, slippage).await?;

        let position = self
            .positions
            .entry(mint.to_string())
            .or_insert(Decimal::ZERO);

        *position += amount;

        Ok(())
    }

    pub async fn sell(
        &mut self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        self.executor.sell(mint, amount, priority, slippage).await?;

        if let Some(position) = self.positions.get_mut(mint) {
            *position -= amount;

            if *position <= Decimal::ZERO {
                self.positions.remove(mint);
            }
        }

        Ok(())
    }

    pub async fn sell_percent(
        &mut self,
        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        let sell_amount = match self.positions.get(mint) {
            Some(position) => *position * Decimal::from(amount) / Decimal::from(100),
            None => return Ok(()),
        };

        self.executor
            .sell_percent(mint, amount, priority, slippage)
            .await?;

        if let Some(position) = self.positions.get_mut(mint) {
            *position -= sell_amount;

            if *position <= Decimal::ZERO {
                self.positions.remove(mint);
            }
        }

        Ok(())
    }

    pub async fn sell_all(&mut self, priority: Decimal, slippage: u16) -> anyhow::Result<()> {
        let positions: Vec<(String, Decimal)> = self
            .positions
            .iter()
            .map(|(mint, amount)| (mint.clone(), *amount))
            .collect();

        for (mint, amount) in positions {
            self.executor
                .sell(&mint, amount, priority, slippage)
                .await?;

            if let Some(position) = self.positions.get_mut(&mint) {
                *position -= amount;

                if *position <= Decimal::ZERO {
                    self.positions.remove(&mint);
                }
            }
        }

        Ok(())
    }
}
