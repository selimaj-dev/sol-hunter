use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct NewToken {
    pub mint: String,
    pub name: String,
    pub symbol: String,
    pub uri: String,

    #[serde(rename = "marketCapSol")]
    pub market_cap_sol: f64,

    #[serde(rename = "solAmount")]
    pub sol_amount: f64,

    #[serde(rename = "traderPublicKey")]
    pub trader_public_key: String,

    #[serde(rename = "txType")]
    pub tx_type: String,
}
