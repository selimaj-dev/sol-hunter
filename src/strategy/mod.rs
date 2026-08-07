pub mod veloc;

use std::sync::Arc;

use crate::bot::Bot;

use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct NewToken {
    pub mint: String,
    #[serde(rename = "traderPublicKey")]
    pub trader_public_key: String,

    pub name: String,
    pub symbol: String,
    pub uri: String,
    #[serde(rename = "marketCapSol")]
    pub market_cap_sol: f64,
    #[serde(rename = "solAmount")]
    pub sol_amount: f64,
}

#[derive(Debug, Clone, Deserialize)]
pub enum TradeType {
    #[serde(rename = "buy")]
    Buy,
    #[serde(rename = "sell")]
    Sell,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Trade {
    pub signature: String,
    pub mint: String,

    #[serde(rename = "traderPublicKey")]
    pub trader: String,

    #[serde(rename = "txType")]
    pub tx_type: TradeType,

    #[serde(rename = "solAmount")]
    pub sol_amount: f64,

    #[serde(rename = "tokenAmount")]
    pub token_amount: f64,

    #[serde(rename = "marketCapSol")]
    pub market_cap_sol: f64,

    #[serde(rename = "vTokensInBondingCurve")]
    pub v_tokens_in_bonding_curve: f64,

    #[serde(rename = "vSolInBondingCurve")]
    pub v_sol_in_bonding_curve: f64,
}

#[async_trait::async_trait]
pub trait Strategy: Send + Sync {
    async fn execute_sell_all(&mut self, bot: Arc<Bot>) -> anyhow::Result<()>;

    async fn on_new_coin(&mut self, bot: Arc<Bot>, token: NewToken) -> anyhow::Result<()>;

    async fn on_trade(&mut self, bot: Arc<Bot>, trade: Trade) -> anyhow::Result<()>;
}
