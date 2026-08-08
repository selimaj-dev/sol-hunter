use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

use log::{debug, info, trace, warn};
use rust_decimal::{Decimal, dec};

use crate::bot::Bot;
use crate::data::tradelog::{ExitReason, TradeLog};
use crate::data::{NewToken, Trade, TradeType};
use crate::strategy::Strategy;

const BUY_AMOUNT_SOL: Decimal = dec!(0.2);
const PRIORITY: Decimal = dec!(0.0002);
const SLIPPAGE: u16 = 10;
const MAX_SUBSCRIBED_TOKENS: usize = 25;
const MAX_OPEN_POSITIONS: usize = 5;

// Strict entry filters
const MIN_UNIQUE_BUYERS: usize = 0;
const MIN_NET_SOL_FLOW: f64 = 0.0003;
const MIN_TRADE_COUNT: usize = 0;
const MAX_CURVE_SOL: f64 = 800.0;
const MIN_MOMENTUM_PCT: f64 = 0.001;

// Exit rules
const TAKE_PROFIT_PCT: f64 = 0.40;
const STOP_LOSS_PCT: f64 = -0.10;
const TRAILING_STOP_DROP_PCT: f64 = 0.12;
const TRAILING_STOP_MIN_GAIN_PCT: f64 = 0.10;
const STALL_DURATION: Duration = Duration::from_secs(2);

struct TokenTracker {
    created_at: Instant,
    first_price_sol: Option<f64>,
    unique_buyers: HashSet<String>,
    net_sol_flow: f64,
    trade_count: usize,
}

struct OpenPosition {
    trade: TradeLog,

    highest_price_sol: f64,
    last_price_sol: f64,
    last_high_time: Instant,
}

pub struct MomentumVelocityStrategy {
    max_tracking_duration: Duration,

    trackers: HashMap<String, TokenTracker>,
    positions: HashMap<String, OpenPosition>,

    // Tracks active token subscriptions to enforce <= 5 limit
    active_subscriptions: VecDeque<String>,
}

impl Default for MomentumVelocityStrategy {
    fn default() -> Self {
        Self {
            max_tracking_duration: Duration::from_secs(45),
            trackers: HashMap::new(),
            positions: HashMap::new(),
            active_subscriptions: VecDeque::with_capacity(MAX_SUBSCRIBED_TOKENS),
        }
    }
}

impl MomentumVelocityStrategy {
    pub fn new() -> Self {
        Self::default()
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
        bot.executor.unsubscribe(mint).await;
        // if let Err(e) = bot.executor.unsubscribe(mint).await {
        //     warn!("[{}] Unsubscribe request failed: {:?}", mint, e);
        // }
        self.trackers.remove(mint);
        self.active_subscriptions.retain(|m| m != mint);
        Ok(())
    }

    async fn execute_exit(
        &mut self,
        bot: &Arc<Bot>,
        mint: &str,
        reason: ExitReason,
    ) -> anyhow::Result<()> {
        let Some((entry_price, exit_price)) = self
            .positions
            .get(mint)
            .map(|pos| (pos.trade.entry_price_sol, pos.last_price_sol))
        else {
            return Ok(());
        };

        let pnl = ((exit_price - entry_price) / entry_price) * 100.0;

        info!(
            "[{}] EXECUTING SELL {}% {:?}",
            mint, pnl, reason,
        );

        bot.executor.sell(mint, 100, PRIORITY, SLIPPAGE).await?;

        if let Some(mut pos) = self.positions.remove(mint) {
            pos.trade.close(exit_price, reason);

            bot.trade_log.lock().await.push(pos.trade);
        }

        self.cleanup_and_unsubscribe(bot, mint).await?;
        Ok(())
    }
}

#[async_trait::async_trait]
impl Strategy for MomentumVelocityStrategy {
    async fn execute_sell_all(&mut self, bot: Arc<Bot>) -> anyhow::Result<()> {
        bot.executor.sell_all(PRIORITY, SLIPPAGE).await
    }

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
                    debug!(
                        "[{}] Capacity reached ({}/{}). Evicting un-bought token from queue.",
                        mint_to_remove, MAX_SUBSCRIBED_TOKENS, MAX_SUBSCRIBED_TOKENS
                    );
                    bot.executor.unsubscribe(&mint_to_remove).await;
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

        debug!("[{}] Subscribing and creating tracker.", token.mint);
        bot.executor.subscribe(&token.mint).await;
        self.active_subscriptions.push_back(token.mint.clone());

        self.trackers.insert(
            token.mint,
            TokenTracker {
                created_at: Instant::now(),
                first_price_sol: None,
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
        // 1. Price-based exits for the mint of this trade.
        //    Updates the highest price / stall timer before any stall scan.
        // -------------------------------------------------------------
        let mut exits: Vec<(String, ExitReason)> = Vec::new();

        if let Some(pos) = self.positions.get_mut(mint) {
            pos.last_price_sol = current_price;

            if current_price > pos.highest_price_sol {
                pos.highest_price_sol = current_price;
                pos.last_high_time = Instant::now();
                trace!("[{}] New high reached: {:.9} SOL", mint, current_price);
            }

            let price_change_pct =
                (current_price - pos.trade.entry_price_sol) / pos.trade.entry_price_sol;
            let drop_from_peak = (pos.highest_price_sol - current_price) / pos.highest_price_sol;

            let reason = match () {
                _ if price_change_pct >= TAKE_PROFIT_PCT => Some(ExitReason::TakeProfit),
                _ if price_change_pct <= STOP_LOSS_PCT => Some(ExitReason::StopLoss),
                _ if drop_from_peak >= TRAILING_STOP_DROP_PCT
                    && price_change_pct > TRAILING_STOP_MIN_GAIN_PCT =>
                {
                    Some(ExitReason::TrailingStop)
                }
                _ => None,
            };

            if let Some(reason) = reason {
                exits.push((mint.clone(), reason));
            }
        }

        // -------------------------------------------------------------
        // 2. Stall exits for ALL open positions. Stalled positions stop
        //    producing trades, so this must NOT be gated on the incoming
        //    trade's mint — any trade evaluates every position.
        // -------------------------------------------------------------
        for (position_mint, pos) in self.positions.iter() {
            if pos.last_high_time.elapsed() >= STALL_DURATION {
                exits.push((position_mint.clone(), ExitReason::MomentumStalled));
            }
        }

        // -------------------------------------------------------------
        // 3. Execute any pending exits. Duplicates are no-ops since the
        //    position is removed on the first exit.
        // -------------------------------------------------------------
        for (exit_mint, reason) in exits {
            self.execute_exit(&bot, &exit_mint, reason).await?;
        }

        // -------------------------------------------------------------
        // 4. Evaluate Potential Buys
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

            if tracker.first_price_sol.is_none() {
                tracker.first_price_sol = Some(current_price);
            }

            match trade.tx_type {
                TradeType::Buy => {
                    tracker.unique_buyers.insert(trade.trader.clone());
                    tracker.net_sol_flow += trade.sol_amount;
                }
                TradeType::Sell => {
                    tracker.net_sol_flow -= trade.sol_amount;
                }
            }

            let v_sol = trade.v_sol_in_bonding_curve / 1_000_000_000.0;
            let price_change_pct = match tracker.first_price_sol {
                Some(first_price) if first_price > 0.0 => {
                    (current_price - first_price) / first_price
                }
                _ => 0.0,
            };

            let has_enough_buyers = tracker.unique_buyers.len() >= MIN_UNIQUE_BUYERS;
            let has_volume_surge = tracker.net_sol_flow >= MIN_NET_SOL_FLOW;
            let has_min_trades = tracker.trade_count >= MIN_TRADE_COUNT;
            let is_early_curve = v_sol < MAX_CURVE_SOL;
            let is_buy_trade = matches!(trade.tx_type, TradeType::Buy);
            let has_momentum = price_change_pct >= MIN_MOMENTUM_PCT;
            let has_capacity = self.positions.len() < MAX_OPEN_POSITIONS;

            // Log detailed status of buy criteria evaluation on every trade
            debug!(
                "[{}] Trade #{} ({:?}) | Buyers: {}/{} [{}] | Net Flow: {:.3}/{:.3} SOL [{}] | Trades: {}/{} [{}] | Curve SOL: {:.2} < {:.0} [{}] | Price Δ: {:.2}% >= {:.0}% [{}] | Capacity: {}/{} [{}]",
                mint,
                tracker.trade_count,
                trade.tx_type,
                tracker.unique_buyers.len(),
                MIN_UNIQUE_BUYERS,
                if has_enough_buyers { "PASS" } else { "FAIL" },
                tracker.net_sol_flow,
                MIN_NET_SOL_FLOW,
                if has_volume_surge { "PASS" } else { "FAIL" },
                tracker.trade_count,
                MIN_TRADE_COUNT,
                if has_min_trades { "PASS" } else { "FAIL" },
                v_sol,
                MAX_CURVE_SOL,
                if is_early_curve { "PASS" } else { "FAIL" },
                price_change_pct * 100.0,
                MIN_MOMENTUM_PCT * 100.0,
                if has_momentum { "PASS" } else { "FAIL" },
                self.positions.len(),
                MAX_OPEN_POSITIONS,
                if has_capacity { "PASS" } else { "FAIL" }
            );

            if has_capacity
                && has_enough_buyers
                && has_volume_surge
                && has_min_trades
                && is_buy_trade
                && has_momentum
                && is_early_curve
            {
                info!(
                    "🚀 BUY SIGNAL TRIGGERED for {}! Unique Buyers: {}, Net Flow: {:.3} SOL, Trades: {}, Curve SOL: {:.2}, Price Δ: {:.2}%",
                    mint,
                    tracker.unique_buyers.len(),
                    tracker.net_sol_flow,
                    tracker.trade_count,
                    trade.v_sol_in_bonding_curve,
                    price_change_pct * 100.0
                );

                bot.executor
                    .buy(mint, BUY_AMOUNT_SOL, PRIORITY, SLIPPAGE)
                    .await?;

                self.positions.insert(
                    mint.clone(),
                    OpenPosition {
                        trade: TradeLog::new(
                            mint.clone(),
                            current_price,
                            tracker.unique_buyers.len(),
                            tracker.net_sol_flow,
                            trade.v_sol_in_bonding_curve,
                        ),
                        highest_price_sol: current_price,
                        last_price_sol: current_price,
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
