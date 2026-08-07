use anyhow::{Context, anyhow};
use reqwest::Client;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::executor::Executor;

const TRADE_LIGHTNING_URL: &str = "https://pumpdev.io/api/trade-lightning";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PumpDevAccount {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[derive(Debug, Deserialize)]
struct TradeResponse {
    signature: String,
}

pub struct PumpDev {
    pub account: Mutex<PumpDevAccount>,
    pub client: Client,
}

impl PumpDev {
    pub fn new(account: PumpDevAccount) -> Self {
        Self {
            account: Mutex::new(account),
            client: Client::new(),
        }
    }

    async fn trade(
        &self,
        action: &str,
        mint: &str,
        amount: String,
        priority: Decimal,
        slippage: u16,
        denominated_in_sol: bool,
    ) -> anyhow::Result<()> {
        let account = self.account.lock().await;

        let response = self
            .client
            .post(format!("{TRADE_LIGHTNING_URL}?api-key={}", account.api_key))
            .json(&serde_json::json!({
                "action": action,
                "mint": mint,
                "amount": amount,
                "denominatedInSol": if denominated_in_sol { "true" } else { "false" },
                "slippage": slippage,
                "priorityFee": priority,
            }))
            .send()
            .await
            .context("Failed to send trade request")?;

        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            let error = serde_json::from_str::<serde_json::Value>(&body)
                .ok()
                .and_then(|value| value.get("error").cloned())
                .and_then(|value| value.as_str().map(str::to_string))
                .unwrap_or_else(|| body.trim().to_string());

            return Err(anyhow!("Trade failed ({status}): {error}"));
        }

        let data: TradeResponse =
            serde_json::from_str(&body).context("Failed to parse trade response")?;

        log::info!("{action} executed: {}", data.signature);

        Ok(())
    }
}

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
