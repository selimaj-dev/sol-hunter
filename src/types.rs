use std::str::FromStr;

use serde::{Deserialize, Deserializer};
use serde_json::Value;
use solana_sdk::pubkey::Pubkey;

#[derive(Debug)]
pub enum PumpDevEvent {
    Connected { client_id: u64, message: String },
    ConnectionStatus { connected: bool, timestamp: u64 },
    Subscribed { method: String },
    Create(NewToken),
}

impl<'de> Deserialize<'de> for PumpDevEvent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;

        if value.get("txType").is_some() {
            let token: NewToken =
                serde_json::from_value(value).map_err(serde::de::Error::custom)?;

            return Ok(PumpDevEvent::Create(token));
        }

        #[derive(Deserialize)]
        #[serde(tag = "type")]
        enum Tagged {
            #[serde(rename = "connected")]
            Connected {
                #[serde(rename = "clientId")]
                client_id: u64,
                message: String,
            },

            #[serde(rename = "connectionStatus")]
            ConnectionStatus { connected: bool, timestamp: u64 },

            #[serde(rename = "subscribed")]
            Subscribed { method: String },
        }

        match serde_json::from_value(value).map_err(serde::de::Error::custom)? {
            Tagged::Connected { client_id, message } => {
                Ok(PumpDevEvent::Connected { client_id, message })
            }
            Tagged::ConnectionStatus {
                connected,
                timestamp,
            } => Ok(PumpDevEvent::ConnectionStatus {
                connected,
                timestamp,
            }),
            Tagged::Subscribed { method } => Ok(PumpDevEvent::Subscribed { method }),
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct NewToken {
    pub mint: PublicKeyParseable,
    #[serde(rename = "traderPublicKey")]
    pub trader_public_key: PublicKeyParseable,

    pub name: String,
    pub symbol: String,
    pub uri: String,
    #[serde(rename = "marketCapSol")]
    pub market_cap_sol: f64,
    #[serde(rename = "solAmount")]
    pub sol_amount: f64,
}

impl std::ops::Deref for PublicKeyParseable {
    type Target = Pubkey;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for PublicKeyParseable {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug, Clone)]
pub struct PublicKeyParseable(pub Pubkey);

impl<'de> Deserialize<'de> for PublicKeyParseable {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;

        Pubkey::from_str(&value)
            .map(PublicKeyParseable)
            .map_err(serde::de::Error::custom)
    }
}
