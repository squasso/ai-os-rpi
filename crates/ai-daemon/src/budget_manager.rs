use chrono::{Local, Datelike};
use common::types::{BudgetStatus, Model};
use crate::BudgetConfig;
use anyhow::Result;

pub struct BudgetManager {
    config:        BudgetConfig,
    spent_today:   f64,
    spent_month:   f64,
    today_day:     u32,
    today_month:   u32,
}

impl BudgetManager {
    pub fn new(config: &BudgetConfig) -> Result<Self> {
        let now = Local::now();
        Ok(Self {
            config: BudgetConfig {
                monthly_limit_usd:    config.monthly_limit_usd,
                daily_soft_limit_usd: config.daily_soft_limit_usd,
                alert_at_percent:     config.alert_at_percent,
            },
            spent_today:  0.0,
            spent_month:  0.0,
            today_day:    now.day(),
            today_month:  now.month(),
        })
    }

    /// Registra i token usati e aggiorna i contatori
    pub fn record(&mut self, tokens_in: u32, tokens_out: u32, model: &Model) {
        self.roll_if_needed();
        let (cost_in, cost_out) = model.cost_per_mtok();
        let cost = (tokens_in  as f64 / 1_000_000.0) * cost_in
                 + (tokens_out as f64 / 1_000_000.0) * cost_out;
        self.spent_today += cost;
        self.spent_month += cost;
    }

    pub fn is_blocked(&self) -> bool {
        self.spent_month >= self.config.monthly_limit_usd
    }

    pub fn status(&self) -> BudgetStatus {
        let pct = self.spent_month / self.config.monthly_limit_usd * 100.0;
        BudgetStatus {
            spent_today_usd:  self.spent_today,
            spent_month_usd:  self.spent_month,
            limit_month_usd:  self.config.monthly_limit_usd,
            limit_daily_usd:  self.config.daily_soft_limit_usd,
            warning:          pct >= self.config.alert_at_percent as f64,
            blocked:          self.is_blocked(),
        }
    }

    /// Azzera contatori se è cambiato giorno/mese
    fn roll_if_needed(&mut self) {
        let now = Local::now();
        if now.month() != self.today_month {
            self.spent_month = 0.0;
            self.spent_today = 0.0;
            self.today_month = now.month();
            self.today_day   = now.day();
        } else if now.day() != self.today_day {
            self.spent_today = 0.0;
            self.today_day   = now.day();
        }
    }
}
