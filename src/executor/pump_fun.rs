use rust_decimal::Decimal;

use crate::executor::Executor;

pub struct PumpDev {}

#[async_trait::async_trait]
impl Executor for PumpDev {
    async fn buy(
        &self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        log::info!("BUY {mint} {amount} SOL");
        Ok(())
        // self.trade(
        //     "buy",
        //     mint,
        //     amount.round_dp(3).to_string(),
        //     priority,
        //     slippage,
        //     true,
        // )
        // .await
    }

    async fn sell(
        &self,
        mint: &str,
        amount: Decimal,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        log::info!("SELL {mint} {amount} SOL");
        Ok(())
        // self.trade(
        //     "sell",
        //     mint,
        //     amount.round_dp(3).to_string(),
        //     priority,
        //     slippage,
        //     false,
        // )
        // .await
    }

    async fn sell_percent(
        &self,
        mint: &str,
        amount: u8,
        priority: Decimal,
        slippage: u16,
    ) -> anyhow::Result<()> {
        log::info!("SELL {mint} {amount}%");
        Ok(())
        // self.trade(
        //     "sell",
        //     mint,
        //     format!("{amount}%"),
        //     priority,
        //     slippage,
        //     false,
        // )
        // .await
    }
}
