use std::collections::HashMap;

use rust_decimal::Decimal;

use crate::launchpad::Client;
use crate::launchpad::Launchpad;

pub struct PumpFun {
    pub positions: HashMap<String, Decimal>,
}

impl PumpFun {
    pub fn new() -> Self {
        Self {
            positions: HashMap::new(),
        }
    }
}

#[allow(unused_variables)]
#[async_trait::async_trait]
impl Launchpad for PumpFun {
    async fn buy(
        &mut self,
        client: &Client,

        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sell(
        &mut self,
        client: &Client,

        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    fn get_positions<'a>(&'a self) -> HashMap<String, Decimal> {
        self.positions.clone()
    }
}
