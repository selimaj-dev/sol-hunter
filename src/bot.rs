use std::sync::Arc;

use anyhow::Context;

use tokio::sync::{Mutex, mpsc, watch};

use crate::{
    data::{
        NewToken,
        account::{Account, AccountManager},
        tradelog::TradeLog,
    },
    launchpad::Executor,
    strategy::{Strategy, veloc::MomentumVelocityStrategy},
};

pub struct Bot {
    pub executor: Arc<Executor>,
    pub strategy: Mutex<Box<dyn Strategy>>,
    pub account_manager: Mutex<AccountManager>,
    pub current_account: Mutex<Account>,
    pub trade_log: Mutex<Vec<TradeLog>>,
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
            executor: Executor::new(&accounts.api_key).await?,
            strategy: Mutex::new(Box::new(MomentumVelocityStrategy::new())),
            account_manager: Mutex::new(accounts),
            current_account: Mutex::new(account),
            trade_log: Mutex::new(Vec::new()),
        }))
    }

    pub async fn start(
        self: &Arc<Self>,
        mut shutdown: watch::Receiver<bool>,
    ) -> anyhow::Result<()> {
        let mut rx = self.executor.listen().await?;

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

                result = self.tick(&mut rx) => {
                    if result? {
                        log::warn!("Websocket closed.");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn tick(self: &Arc<Self>, rx: &mut mpsc::Receiver<NewToken>) -> anyhow::Result<bool> {
        if let Some(token) = rx.recv().await {
            self.strategy
                .lock()
                .await
                .on_new_coin(self.clone(), token)
                .await?;

            Ok(false)
        } else {
            Ok(true)
        }
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
