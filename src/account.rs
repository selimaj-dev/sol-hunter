use std::{collections::HashMap, path::PathBuf};

use crate::executor::pump_fun::PumpDevAccount;
use anyhow::{Context, anyhow};
use serde::{Deserialize, Serialize};

pub fn get_accounts_path() -> anyhow::Result<PathBuf> {
    Ok(std::env::home_dir()
        .context("Failed to get home dir")?
        .join(".config")
        .join("sol-hunter")
        .join("accounts.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Account {
    PumpDev(PumpDevAccount),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountManager {
    pub active: String,
    pub accounts: HashMap<String, Account>,
}

impl Account {
    pub async fn new() -> anyhow::Result<Self> {
        let response = reqwest::Client::new()
            .post("https://pumpdev.io/api/wallet/create")
            .json(&serde_json::json!({}))
            .send()
            .await
            .context("Failed to create PumpDev wallet")?;

        if !response.status().is_success() {
            let status = response.status();
            let body: serde_json::Value = response.json().await.unwrap_or_default();
            let message = body
                .get("error")
                .and_then(serde_json::Value::as_str)
                .unwrap_or("Unknown error");
            return Err(anyhow!(
                "Failed to create PumpDev wallet ({status}): {message}"
            ));
        }

        let account: PumpDevAccount = response
            .json()
            .await
            .context("Failed to parse PumpDev wallet")?;

        Ok(Self::PumpDev(account))
    }
}

impl AccountManager {
    pub async fn get() -> anyhow::Result<Self> {
        let path = get_accounts_path()?;

        if !path.exists() {
            let default = Self::new().await?;

            tokio::fs::create_dir_all(path.parent().unwrap()).await?;
            tokio::fs::write(&path, default.to_string()?).await?;

            return Ok(default);
        }

        let output = tokio::fs::read_to_string(path).await?;

        Self::from_str(&output)
    }

    pub async fn new() -> anyhow::Result<Self> {
        let mut accounts = HashMap::new();
        accounts.insert("default".to_string(), Account::new().await?);

        Ok(Self {
            active: "default".to_string(),
            accounts,
        })
    }

    pub fn from_str(s: &str) -> anyhow::Result<Self> {
        serde_json::from_str(s).context("Failed to parse accounts")
    }

    pub fn to_string(&self) -> anyhow::Result<String> {
        serde_json::to_string(self).context("Failed to serialize accounts")
    }
}
