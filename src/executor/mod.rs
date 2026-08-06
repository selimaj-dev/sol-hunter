pub mod pump_fun;

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::executor::pump_fun::PumpDevAccount;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Account {
    PumpDev(PumpDevAccount),
}

#[allow(async_fn_in_trait, unused_variables)]
pub trait Executor {
    async fn buy(
        &self,
        mint: String,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sell(
        &self,
        mint: String,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }
}
