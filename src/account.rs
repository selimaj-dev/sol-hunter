use std::{collections::HashMap, path::PathBuf};

use anyhow::Context;
use serde::{Deserialize, Serialize};

pub fn get_accounts_path() -> anyhow::Result<PathBuf> {
    Ok(std::env::home_dir()
        .context("Failed to get home dir")?
        .join(".config")
        .join("sol-hunter")
        .join("accounts.json"))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Account {
    #[serde(rename = "publicKey")]
    pub public_key: String,
    #[serde(rename = "privateKey")]
    pub private_key: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccountManager {
    #[serde(rename = "apiKey")]
    pub api_key: String,
    pub active: String,
    pub accounts: HashMap<String, Account>,
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
        accounts.insert(
            "default".to_string(),
            Account {
                public_key: String::new(),
                private_key: String::new(),
            },
        );

        Ok(Self {
            active: "default".to_string(),
            api_key: String::new(),
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
