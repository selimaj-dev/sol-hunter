use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::executor::Executor;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpDevAccount {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

pub struct PumpDev {
    pub account: Mutex<PumpDevAccount>,
}

impl Executor for PumpDev {
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
