use pump_rust_client::PumpSdk;

use crate::executor::Executor;

pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

pub struct PumpFun {
    pub private_key: String,
    pub sdk: PumpSdk,
}

impl Executor for PumpFun {
    async fn buy(&self, token: solana_sdk::pubkey::Pubkey, amount: f64) -> anyhow::Result<()> {
        log::info!("Buying {token}");

        // self.sdk.buy_v2_instructions(
        //     global,
        //     bonding_curve,
        //     base_mint,
        //     quote_token_program,
        //     user,
        //     amount,
        //     max_quote_tokens,
        // );

        Ok(())
    }

    async fn sell(&self, token: solana_sdk::pubkey::Pubkey, amount: f64) -> anyhow::Result<()> {
        log::info!("Selling {token}");

        Ok(())
    }
}
