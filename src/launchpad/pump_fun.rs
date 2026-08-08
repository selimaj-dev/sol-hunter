use std::collections::HashMap;
use std::sync::Arc;

use base64::{Engine, engine::general_purpose::STANDARD};
use futures_util::SinkExt;
use futures_util::StreamExt;
use rust_decimal::Decimal;
use serde::Deserialize;
use tokio::sync::mpsc;
use tokio_tungstenite::tungstenite::Message;

use crate::data::Event;
use crate::data::NewToken;
use crate::data::Trade;
use crate::data::TradeType;
use crate::launchpad::Client;
use crate::launchpad::Launchpad;

const PUMP_FUN_PROGRAM: &str = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";
const CREATE_EVENT_DISCRIMINATOR: [u8; 8] = [27, 114, 169, 77, 222, 235, 99, 118];
const TRADE_EVENT_DISCRIMINATOR: [u8; 8] = [189, 219, 127, 211, 78, 230, 97, 238];

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

    async fn listen(client: Arc<Client>, tx: mpsc::Sender<Event>) -> anyhow::Result<()> {
        let mut ws = client.solana.lock().await;

        ws.send(
            serde_json::json!({
                "jsonrpc": "2.0",
                "id": 1,
                "method": "logsSubscribe",
                "params": [
                    { "mentions": [PUMP_FUN_PROGRAM] },
                    { "commitment": "processed" }
                ]
            })
            .to_string()
            .into(),
        )
        .await?;

        log::info!(
            "[PUMP.FUN] Listening for new tokens from {}",
            PUMP_FUN_PROGRAM
        );

        while let Some(message) = ws.next().await {
            let Message::Text(text) = message? else {
                continue;
            };

            let Ok(notification) = serde_json::from_str::<LogsNotification>(&text) else {
                continue;
            };

            if notification.method != "logsNotification" {
                continue;
            }

            let logs = notification.params.result.value.logs;

            if !logs
                .iter()
                .any(|log| log.starts_with("Program log: Instruction: Create"))
            {
                continue;
            }

            for log in &logs {
                let Some(encoded) = log.strip_prefix("Program data: ") else {
                    continue;
                };

                let Ok(data) = STANDARD.decode(encoded) else {
                    continue;
                };

                if let Some(token) = parse_create_event(&data) {
                    tx.send(Event::NewToken(token)).await?;
                }

                if let Some(trade) =
                    parse_trade_event(&data, notification.params.result.value.signature.clone())
                {
                    if client.subscribed.lock().await.contains(&trade.mint) {
                        tx.send(Event::Trade(trade)).await?;
                    }
                }
            }
        }

        Ok(())
    }

    fn get_positions<'a>(&'a self) -> HashMap<String, Decimal> {
        self.positions.clone()
    }
}

fn parse_trade_event(data: &[u8], signature: String) -> Option<Trade> {
    if data.len() < 8 || data[..8] != TRADE_EVENT_DISCRIMINATOR {
        return None;
    }

    let mut offset = 8;

    let mint = decode_pubkey(data, &mut offset)?;

    let sol_amount = decode_u64(data, &mut offset)? as f64;
    let token_amount = decode_u64(data, &mut offset)? as f64;

    let is_buy = decode_bool(data, &mut offset)?;

    let trader = decode_pubkey(data, &mut offset)?;

    // timestamp exists but we don't need it currently
    let _timestamp = decode_i64(data, &mut offset)?;

    let v_sol_in_bonding_curve = decode_u64(data, &mut offset)? as f64;

    let v_tokens_in_bonding_curve = decode_u64(data, &mut offset)? as f64;

    // These exist in the event but aren't needed for your struct
    let _real_sol_reserves = decode_u64(data, &mut offset)?;

    let _real_token_reserves = decode_u64(data, &mut offset)?;

    Some(Trade {
        signature,
        mint,
        trader,
        tx_type: if is_buy {
            TradeType::Buy
        } else {
            TradeType::Sell
        },
        sol_amount,
        token_amount,

        // Pump.fun bonding curve market cap:
        // token_price = virtual SOL / virtual tokens
        // market cap is derived
        market_cap_sol: if v_tokens_in_bonding_curve > 0.0 {
            v_sol_in_bonding_curve / v_tokens_in_bonding_curve
        } else {
            0.0
        },

        v_tokens_in_bonding_curve,
        v_sol_in_bonding_curve,
    })
}

fn parse_create_event(data: &[u8]) -> Option<NewToken> {
    if data.len() < 8 || data[..8] != CREATE_EVENT_DISCRIMINATOR {
        return None;
    }

    let mut offset = 8;

    let name = decode_string(data, &mut offset)?;
    let symbol = decode_string(data, &mut offset)?;
    let uri = decode_string(data, &mut offset)?;
    let mint = decode_pubkey(data, &mut offset)?;
    let _bonding_curve = decode_pubkey(data, &mut offset)?;
    let trader_public_key = decode_pubkey(data, &mut offset)?;
    let _creator = decode_pubkey(data, &mut offset)?;

    // Virtual reserves were added in a later program version; fall back to 0.0 for legacy events.
    let market_cap_sol = decode_market_cap(data, &mut offset).unwrap_or(0.0);

    Some(NewToken {
        name,
        symbol,
        uri,
        mint,
        trader_public_key,
        market_cap_sol,
    })
}

fn decode_market_cap(data: &[u8], offset: &mut usize) -> Option<f64> {
    let _timestamp = decode_i64(data, offset)?;
    let virtual_token_reserves = decode_u64(data, offset)?;
    let virtual_sol_reserves = decode_u64(data, offset)?;
    let _real_token_reserves = decode_u64(data, offset)?;
    let token_total_supply = decode_u64(data, offset)?;

    if virtual_token_reserves == 0 {
        return Some(0.0);
    }

    Some(
        virtual_sol_reserves as f64 / 1e9 * token_total_supply as f64
            / virtual_token_reserves as f64,
    )
}

fn decode_string(data: &[u8], offset: &mut usize) -> Option<String> {
    let len = u32::from_le_bytes(data.get(*offset..*offset + 4)?.try_into().ok()?) as usize;
    *offset += 4;

    let bytes = data.get(*offset..*offset + len)?;
    *offset += len;

    String::from_utf8(bytes.to_vec()).ok()
}

fn decode_pubkey(data: &[u8], offset: &mut usize) -> Option<String> {
    let bytes = data.get(*offset..*offset + 32)?;
    *offset += 32;

    Some(bs58::encode(bytes).into_string())
}

fn decode_u64(data: &[u8], offset: &mut usize) -> Option<u64> {
    let bytes = data.get(*offset..*offset + 8)?.try_into().ok()?;
    *offset += 8;

    Some(u64::from_le_bytes(bytes))
}

fn decode_i64(data: &[u8], offset: &mut usize) -> Option<i64> {
    let bytes = data.get(*offset..*offset + 8)?.try_into().ok()?;
    *offset += 8;

    Some(i64::from_le_bytes(bytes))
}

fn decode_bool(data: &[u8], offset: &mut usize) -> Option<bool> {
    if *offset >= data.len() {
        return None;
    }

    let value = data[*offset] != 0;
    *offset += 1;

    Some(value)
}

#[derive(Debug, Deserialize)]
struct LogsNotification {
    method: String,
    params: NotificationParams,
}

#[derive(Debug, Deserialize)]
struct NotificationParams {
    result: NotificationResult,
}

#[derive(Debug, Deserialize)]
struct NotificationResult {
    value: LogsValue,
}

#[derive(Debug, Deserialize)]
struct LogsValue {
    signature: String,
    logs: Vec<String>,
}
