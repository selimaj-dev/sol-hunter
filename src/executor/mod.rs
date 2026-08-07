pub mod pump_fun;

use rust_decimal::Decimal;

use crate::account::Account;

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
