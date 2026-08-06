use pump_rust_client::{
    AccountWrapper, PumpSdk, pump::pump::accounts::BondingCurve, state::Global,
};

use crate::executor::Executor;

pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

pub struct PumpFun {
    pub private_key: String,
    pub sdk: PumpSdk,

    pub global: Global,
}

impl Executor for PumpFun {
    async fn buy(&self, token: solana_sdk::pubkey::Pubkey, amount: u64) -> anyhow::Result<()> {
        log::info!("Buying {token}");

        // let bonding_curve: AccountWrapper<BondingCurve> = _;

        // self.sdk.buy_v2_instructions(
        //     &self.global,
        //     &bonding_curve,
        //     token,
        //     quote_token_program,
        //     user,
        //     amount,
        //     max_quote_tokens,
        // );

        Ok(())
    }

    async fn sell(&self, token: solana_sdk::pubkey::Pubkey, amount: u64) -> anyhow::Result<()> {
        log::info!("Selling {token}");

        Ok(())
    }
}
