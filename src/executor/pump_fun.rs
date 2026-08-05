use crate::executor::Executor;

pub const PROGRAM_ID: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

pub struct PumpFun {
    pub private_key: String,
}

impl Executor for PumpFun {
    async fn buy(&self, token: solana_sdk::pubkey::Pubkey, amount: f64) -> anyhow::Result<()> {
        Ok(())
    }

    async fn sell(&self, token: solana_sdk::pubkey::Pubkey, amount: f64) -> anyhow::Result<()> {
        Ok(())
    }
}
