use std::sync::Arc;

use anyhow::{Context, anyhow};
use helius::Helius;

use tokio::sync::{Mutex, watch};

use crate::{
    data::account::{Account, AccountManager},
    data::tradelog::TradeLog,
    strategy::{Strategy, veloc::MomentumVelocityStrategy},
};

pub struct Bot {
    pub account_manager: Mutex<AccountManager>,
    pub current_account: Mutex<Account>,

    pub strategy: Mutex<Box<dyn Strategy>>,

    pub executor: Mutex<()>,
    pub trade_log: Mutex<Vec<TradeLog>>,

    pub helius: Helius,
}

impl Bot {
    pub async fn new() -> anyhow::Result<Arc<Self>> {
        let accounts = AccountManager::get().await?;

        let account = accounts
            .accounts
            .get(&accounts.active)
            .context("Failed to get account")?
            .clone();

        Ok(Arc::new(Self {
            helius: Helius::new_async(&accounts.api_key, helius::types::Cluster::Devnet).await?,

            account_manager: Mutex::new(accounts),
            current_account: Mutex::new(account),
            strategy: Mutex::new(Box::new(MomentumVelocityStrategy::new())),

            trade_log: Mutex::new(Vec::new()),
            executor: Mutex::new(()),
        }))
    }

    pub async fn start(
        self: &Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        loop {
            tokio::select! {
                _ = shutdown.changed() => {
                    if *shutdown.borrow() {
                        log::info!("Shutdown signal received.");

                        self.strategy
                            .lock()
                            .await
                            .execute_sell_all(self.clone())
                            .await?;

                        log::info!("Bye!");

                        break;
                    }
                }

                result = self.tick() => {
                    if result? {
                        log::warn!("Websocket closed.");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn tick(self: &Arc<Self>) -> anyhow::Result<bool> {
        let ws = self
            .helius
            .ws()
            .ok_or_else(|| anyhow!("Unable to obtain ws"))?;

        // drop(ws);
        // self.strategy
        //     .lock()
        //     .await
        //     .on_new_coin(self.clone(), token)
        //     .await?;

        // drop(ws);
        // self.strategy
        //     .lock()
        //     .await
        //     .on_trade(self.clone(), trade)
        //     .await?;

        Ok(false)
    }
}

impl Bot {
    pub async fn subscribe(&self, mint: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn unsubscribe(&self, mint: &str) -> anyhow::Result<()> {
        Ok(())
    }

    pub async fn refresh_account(self: &Arc<Self>) -> anyhow::Result<()> {
        self.strategy
            .lock()
            .await
            .execute_sell_all(self.clone())
            .await?;

        let accounts = self.account_manager.lock().await;

        let account = accounts
            .accounts
            .get(&accounts.active)
            .context("Failed to get account")?
            .clone();

        *self.current_account.lock().await = account;

        Ok(())
    }
}
