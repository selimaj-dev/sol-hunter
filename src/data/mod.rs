pub mod account;
pub mod tradelog;

use serde::Deserialize;

#[derive(Debug, Clone)]
pub enum Event {
    NewToken(NewToken),
    Trade(Trade),
}

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
