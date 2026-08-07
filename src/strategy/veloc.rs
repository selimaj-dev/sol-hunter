use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::{Duration, Instant};

use rust_decimal::{Decimal, dec};

use crate::bot::Bot;
use crate::strategy::Strategy;
use crate::types::{NewToken, Trade, TradeType};

const BUY_AMOUNT_SOL: Decimal = dec!(0.2);
const PRIORITY: Decimal = dec!(0.0002);
const SLIPPAGE: u16 = 10;

// --- Strategy Implementation ---

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
    // Configurable thresholds
    min_unique_buyers: usize,
    min_net_sol_flow: f64,
    max_tracking_duration: Duration,

    // In-memory state tracking
    trackers: HashMap<String, TokenTracker>,
    positions: HashMap<String, OpenPosition>,
}

impl MomentumVelocityStrategy {
    pub fn new() -> Self {
        Self {
            min_unique_buyers: 4,
            min_net_sol_flow: 1.5,
            max_tracking_duration: Duration::from_secs(45),
            trackers: HashMap::new(),
            positions: HashMap::new(),
        }
    }

    /// Calculate approximate token price in SOL using the bonding curve reserves
    fn calculate_price_sol(&self, trade: &Trade) -> f64 {
        if trade.v_tokens_in_bonding_curve == 0.0 {
            return 0.0;
        }
        trade.v_sol_in_bonding_curve / trade.v_tokens_in_bonding_curve
    }
}

#[async_trait::async_trait]
impl Strategy for MomentumVelocityStrategy {
    async fn on_new_coin(&mut self, _bot: Arc<Bot>, token: NewToken) -> anyhow::Result<()> {
        // Cleanup old untracked tokens to keep memory lean
        self.trackers
            .retain(|_, v| v.created_at.elapsed() < Duration::from_secs(120));

        // Initialize tracking for new token
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

            // Track peak price for trailing stop logic
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
                self.trackers.remove(mint);
            }

            return Ok(());
        }

        // -------------------------------------------------------------
        // 2. Evaluate Potential Buys
        // -------------------------------------------------------------
        if let Some(tracker) = self.trackers.get_mut(mint) {
            // Stop monitoring if the token is older than maximum initial evaluation window
            if tracker.created_at.elapsed() > self.max_tracking_duration {
                self.trackers.remove(mint);
                return Ok(());
            }

            // Update stats
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

            // Check BUY conditions
            let has_enough_buyers = tracker.unique_buyers.len() >= self.min_unique_buyers;
            let has_volume_surge = tracker.net_sol_flow >= self.min_net_sol_flow;

            // Skip if bonding curve is already close to completing (e.g. >30 SOL in curve)
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

                // Stop tracking buy metrics for this token
                self.trackers.remove(mint);
            }
        }

        Ok(())
    }
}
