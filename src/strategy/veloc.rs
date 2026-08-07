use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, info, trace, warn};
use rust_decimal::{Decimal, dec};

use crate::bot::Bot;
use crate::strategy::Strategy;
use crate::types::{NewToken, Trade, TradeType};

const BUY_AMOUNT_SOL: Decimal = dec!(0.2);
const PRIORITY: Decimal = dec!(0.0002);
const SLIPPAGE: u16 = 10;
const MAX_SUBSCRIBED_TOKENS: usize = 5;

struct TokenTracker {
    created_at: Instant,
    unique_buyers: HashSet<String>,
    net_sol_flow: f64,
    trade_count: usize,
}

struct OpenPosition {
    entry_price_sol: f64,
    highest_price_sol: f64,
    last_high_time: Instant,
}

pub struct MomentumVelocityStrategy {
    min_unique_buyers: usize,
    min_net_sol_flow: f64,
    max_tracking_duration: Duration,

    trackers: HashMap<String, TokenTracker>,
    positions: HashMap<String, OpenPosition>,

    // Tracks active token subscriptions to enforce <= 5 limit
    active_subscriptions: VecDeque<String>,
}

impl MomentumVelocityStrategy {
    pub fn new() -> Self {
        Self {
            min_unique_buyers: 1,
            min_net_sol_flow: 0.001,
            max_tracking_duration: Duration::from_secs(45),
            trackers: HashMap::new(),
            positions: HashMap::new(),
            active_subscriptions: VecDeque::with_capacity(MAX_SUBSCRIBED_TOKENS),
        }
    }

    fn calculate_price_sol(&self, trade: &Trade) -> f64 {
        if trade.v_tokens_in_bonding_curve == 0.0 {
            return 0.0;
        }
        trade.v_sol_in_bonding_curve / trade.v_tokens_in_bonding_curve
    }

    /// Internal helper to safely handle unsubscribing and cleaning state
    async fn cleanup_and_unsubscribe(&mut self, bot: &Arc<Bot>, mint: &str) -> anyhow::Result<()> {
        debug!("[{}] Cleaning up state and unsubscribing", mint);
        if let Err(e) = bot.unsubscribe(mint).await {
            warn!("[{}] Unsubscribe request failed: {:?}", mint, e);
        }
        self.trackers.remove(mint);
        self.active_subscriptions.retain(|m| m != mint);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Strategy for MomentumVelocityStrategy {
    async fn on_new_coin(&mut self, bot: Arc<Bot>, token: NewToken) -> anyhow::Result<()> {
        trace!("[NEW COIN] Event received for token: {}", token.mint);

        if self.positions.contains_key(&token.mint) || self.trackers.contains_key(&token.mint) {
            trace!(
                "[{}] Already tracking or holding position. Skipping.",
                token.mint
            );
            return Ok(());
        }

        // Evict oldest tracked token that DOES NOT have an active open position
        while self.active_subscriptions.len() >= MAX_SUBSCRIBED_TOKENS {
            let eviction_index = self
                .active_subscriptions
                .iter()
                .position(|mint| !self.positions.contains_key(mint));

            if let Some(idx) = eviction_index {
                if let Some(mint_to_remove) = self.active_subscriptions.remove(idx) {
                    info!(
                        "[{}] Capacity reached ({}/{}). Evicting un-bought token from queue.",
                        mint_to_remove, MAX_SUBSCRIBED_TOKENS, MAX_SUBSCRIBED_TOKENS
                    );
                    let _ = bot.unsubscribe(&mint_to_remove).await;
                    self.trackers.remove(&mint_to_remove);
                }
            } else {
                warn!(
                    "[QUEUE FULL] All {} slots are occupied by active positions. Cannot track {}",
                    MAX_SUBSCRIBED_TOKENS, token.mint
                );
                return Ok(());
            }
        }

        info!("[{}] Subscribing and creating tracker.", token.mint);
        bot.subscribe(&token.mint).await?;
        self.active_subscriptions.push_back(token.mint.clone());

        self.trackers.insert(
            token.mint,
            TokenTracker {
                created_at: Instant::now(),
                unique_buyers: HashSet::new(),
                net_sol_flow: 0.0,
                trade_count: 0,
            },
        );

        Ok(())
    }

    async fn on_trade(&mut self, bot: Arc<Bot>, trade: Trade) -> anyhow::Result<()> {
        let mint = &trade.mint;
        let current_price = self.calculate_price_sol(&trade);

        // -------------------------------------------------------------
        // 1. Manage Active Positions (Take Profit / Stop Loss / Stall)
        // -------------------------------------------------------------
        if let Some(pos) = self.positions.get_mut(mint) {
            let price_change_pct = (current_price - pos.entry_price_sol) / pos.entry_price_sol;

            if current_price > pos.highest_price_sol {
                pos.highest_price_sol = current_price;
                pos.last_high_time = Instant::now();
                trace!("[{}] New high reached: {:.9} SOL", mint, current_price);
            }

            let drop_from_peak = (pos.highest_price_sol - current_price) / pos.highest_price_sol;

            let (should_sell, reason) = match () {
                _ if price_change_pct >= 0.40 => (
                    true,
                    format!("Take Profit (+{:.1}%)", price_change_pct * 100.0),
                ),
                _ if price_change_pct <= -0.1 => (
                    true,
                    format!("Hard Stop Loss ({:.1}%)", price_change_pct * 100.0),
                ),
                _ if drop_from_peak >= 0.12 && price_change_pct > 0.10 => (
                    true,
                    format!(
                        "Trailing Stop (Peak drop: {:.1}%, gain: +{:.1}%)",
                        drop_from_peak * 100.0,
                        price_change_pct * 100.0
                    ),
                ),
                _ if pos.last_high_time.elapsed() >= Duration::from_secs(25) => {
                    (true, format!("Momentum Stalled (no high for 25s)"))
                }
                _ => (false, String::new()),
            };

            if should_sell {
                info!("[{}] EXECUTING SELL. Reason: {}", mint, reason);
                bot.executor
                    .lock()
                    .await
                    .sell_percent(mint, 100, PRIORITY, SLIPPAGE)
                    .await?;

                self.positions.remove(mint);
                self.cleanup_and_unsubscribe(&bot, mint).await?;
            }

            return Ok(());
        }

        // -------------------------------------------------------------
        // 2. Evaluate Potential Buys
        // -------------------------------------------------------------
        if let Some(tracker) = self.trackers.get_mut(mint) {
            let elapsed = tracker.created_at.elapsed();
            if elapsed > self.max_tracking_duration {
                info!(
                    "[{}] Tracking window expired ({:?} > {:?}). Cleaning up.",
                    mint, elapsed, self.max_tracking_duration
                );
                self.cleanup_and_unsubscribe(&bot, mint).await?;
                return Ok(());
            }

            tracker.trade_count += 1;
            match trade.tx_type {
                TradeType::Buy => {
                    tracker.unique_buyers.insert(trade.trader.clone());
                    tracker.net_sol_flow += trade.sol_amount;
                }
                TradeType::Sell => {
                    tracker.net_sol_flow -= trade.sol_amount;
                }
            }

            let has_enough_buyers = tracker.unique_buyers.len() >= self.min_unique_buyers;
            let has_volume_surge = tracker.net_sol_flow >= self.min_net_sol_flow;
            let v_sol = trade.v_sol_in_bonding_curve / 1_000_000_000.0;
            let is_early_curve = v_sol < 60.0;

            // Log detailed status of buy criteria evaluation on every trade
            debug!(
                "[{}] Trade #{} ({:?}) | Buyers: {}/{} [{}] | Net Flow: {:.3}/{:.3} SOL [{}] | Curve SOL: {:.2} < 60 [{}]",
                mint,
                tracker.trade_count,
                trade.tx_type,
                tracker.unique_buyers.len(),
                self.min_unique_buyers,
                if has_enough_buyers { "PASS" } else { "FAIL" },
                tracker.net_sol_flow,
                self.min_net_sol_flow,
                if has_volume_surge { "PASS" } else { "FAIL" },
                trade.v_sol_in_bonding_curve,
                if is_early_curve { "PASS" } else { "FAIL" }
            );

            if has_enough_buyers && has_volume_surge && is_early_curve {
                info!(
                    "🚀 BUY SIGNAL TRIGGERED for {}! Unique Buyers: {}, Net Flow: {:.3} SOL, Curve SOL: {:.2}",
                    mint,
                    tracker.unique_buyers.len(),
                    tracker.net_sol_flow,
                    trade.v_sol_in_bonding_curve
                );

                bot.executor
                    .lock()
                    .await
                    .buy(mint, BUY_AMOUNT_SOL, PRIORITY, SLIPPAGE)
                    .await?;

                self.positions.insert(
                    mint.clone(),
                    OpenPosition {
                        entry_price_sol: current_price,
                        highest_price_sol: current_price,
                        last_high_time: Instant::now(),
                    },
                );

                self.trackers.remove(mint);
            }
        } else {
            trace!("[{}] Received trade for untracked mint.", mint);
        }

        Ok(())
    }
}
