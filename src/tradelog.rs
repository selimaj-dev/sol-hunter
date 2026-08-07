use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ExitReason {
    TakeProfit,
    StopLoss,
    TrailingStop,
    MomentumStalled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TradeLog {
    pub mint: String,

    pub opened_at: chrono::DateTime<chrono::Utc>,
    pub closed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub duration: Option<chrono::TimeDelta>,

    pub entry_price_sol: f64,
    pub exit_price_sol: Option<f64>,

    pub pnl_percent: Option<f64>,
    pub exit_reason: Option<ExitReason>,

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
            opened_at: chrono::Utc::now(),
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
        let now = chrono::Utc::now();

        let pnl = ((exit_price_sol - self.entry_price_sol) / self.entry_price_sol) * 100.0;

        self.exit_price_sol = Some(exit_price_sol);
        self.pnl_percent = Some(pnl);

        self.closed_at = Some(now);
        self.duration = Some(now.signed_duration_since(self.opened_at));

        self.exit_reason = Some(reason);
    }
}
