use solana_sdk::pubkey::Pubkey;

#[allow(async_fn_in_trait)]
pub trait Executor {
    async fn buy(&self, token: Pubkey, amount: f64) -> anyhow::Result<()>;
    async fn sell(&self, token: Pubkey, amount: f64) -> anyhow::Result<()>;
}
