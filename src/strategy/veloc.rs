use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
            min_unique_buyers: 4,
            min_net_sol_flow: 1.5,
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
        bot.unsubscribe(mint).await?;
        self.trackers.remove(mint);
        self.active_subscriptions.retain(|m| m != mint);
        Ok(())
    }
}

#[async_trait::async_trait]
impl Strategy for MomentumVelocityStrategy {
    async fn on_new_coin(&mut self, bot: Arc<Bot>, token: NewToken) -> anyhow::Result<()> {
        // If we hold an active position in this mint already, skip tracking setup
        if self.positions.contains_key(&token.mint) {
            return Ok(());
        }

        // Enforce maximum 5 active subscriptions
        while self.active_subscriptions.len() >= MAX_SUBSCRIBED_TOKENS {
            if let Some(oldest_mint) = self.active_subscriptions.pop_front() {
                // Do not drop subscription if we currently hold an open position in it
                if self.positions.contains_key(&oldest_mint) {
                    continue;
                }
                bot.unsubscribe(&oldest_mint).await?;
                self.trackers.remove(&oldest_mint);
            }
        }

        // Subscribe to new token
        bot.subscribe(&token.mint).await?;
        self.active_subscriptions.push_back(token.mint.clone());

        // Initialize tracking
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
            }

            let drop_from_peak = (pos.highest_price_sol - current_price) / pos.highest_price_sol;

            let should_sell = match () {
                _ if price_change_pct >= 0.40 => true,  // Take Profit: +40%
                _ if price_change_pct <= -0.15 => true, // Hard Stop Loss: -15%
                _ if drop_from_peak >= 0.12 && price_change_pct > 0.10 => true, // Trailing stop: 12% drop from peak
                _ if pos.last_high_time.elapsed() >= Duration::from_secs(25) => true, // Momentum stalled for 25s
                _ => false,
            };

            if should_sell {
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
            // Unsubscribe & remove if evaluation window expired without a signal
            if tracker.created_at.elapsed() > self.max_tracking_duration {
                self.cleanup_and_unsubscribe(&bot, mint).await?;
                return Ok(());
            }

            // Update trade metrics
            tracker.trade_count += 1;
            match trade.tx_type {
                TradeType::Buy => {
                    tracker.unique_buyers.insert(trade.trader);
                    tracker.net_sol_flow += trade.sol_amount;
                }
                TradeType::Sell => {
                    tracker.net_sol_flow -= trade.sol_amount;
                }
            }

            let has_enough_buyers = tracker.unique_buyers.len() >= self.min_unique_buyers;
            let has_volume_surge = tracker.net_sol_flow >= self.min_net_sol_flow;
            let is_early_curve = trade.v_sol_in_bonding_curve < 60.0;

            if has_enough_buyers && has_volume_surge && is_early_curve {
                bot.executor
                    .lock()
                    .await
                    .buy(mint, BUY_AMOUNT_SOL, PRIORITY, SLIPPAGE)
                    .await?;

                // Register open position
                self.positions.insert(
                    mint.clone(),
                    OpenPosition {
                        entry_price_sol: current_price,
                        highest_price_sol: current_price,
                        last_high_time: Instant::now(),
                    },
                );

                // Stop tracking buy metrics (subscription remains active while holding position)
                self.trackers.remove(mint);
            }
        }

        Ok(())
    }
}
