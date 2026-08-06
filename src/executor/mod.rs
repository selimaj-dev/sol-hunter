pub mod pump_fun;

use rust_decimal::Decimal;

use crate::account::Account;


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

impl Account {
    pub fn executor(self) -> impl Executor {
        match self {
            Self::PumpDev(account) => pump_fun::PumpDev::new(account),
        }
    }
}
