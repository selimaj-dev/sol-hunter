use std::time::{Duration, Instant};

#[derive(Debug, Clone)]
pub enum ExitReason {
    TakeProfit,
    StopLoss,
    TrailingStop,
    MomentumStalled,
}

#[derive(Debug, Clone)]
pub struct TradeLog {
    pub mint: String,

    pub opened_at: Instant,
    pub closed_at: Option<Instant>,

    pub entry_price_sol: f64,
    pub exit_price_sol: Option<f64>,

    pub pnl_percent: Option<f64>,
    pub duration: Option<Duration>,

    pub exit_reason: Option<ExitReason>,

    // Conditions when entered
    pub unique_buyers: usize,
    pub net_sol_flow: f64,
    pub curve_sol: f64,
}

impl TradeLog {
    pub fn new(
        mint: String,
        entry_price_sol: f64,
        unique_buyers: usize,
        net_sol_flow: f64,
        curve_sol: f64,
    ) -> Self {
        Self {
            mint,
            opened_at: Instant::now(),
            closed_at: None,

            entry_price_sol,
            exit_price_sol: None,

            pnl_percent: None,
            duration: None,

            exit_reason: None,

            unique_buyers,
            net_sol_flow,
            curve_sol,
        }
    }

    pub fn close(&mut self, exit_price_sol: f64, reason: ExitReason) {
        let now = Instant::now();

        let pnl = ((exit_price_sol - self.entry_price_sol) / self.entry_price_sol) * 100.0;

        self.exit_price_sol = Some(exit_price_sol);
        self.pnl_percent = Some(pnl);

        self.closed_at = Some(now);
        self.duration = Some(now.duration_since(self.opened_at));

        self.exit_reason = Some(reason);
    }
}
