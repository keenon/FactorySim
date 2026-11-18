//! `fsim-finance`: Equipment financing loans (against resellable machines), credit lines,
//! runway percentiles (10th, 50th, 90th), and bankruptcy detection.

pub mod reference;

use fsim_core::{Money, SimTime};
use serde::{Deserialize, Serialize};

/// An active equipment loan collateralized by a resellable machine on the floor.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EquipmentLoan {
    pub loan_id: String,
    pub collateral_cell_id: String,
    pub original_principal: Money,
    pub remaining_principal: Money,
    pub apr_bps: u32,
    pub monthly_payment: Money,
    pub originated_at: SimTime,
}

/// Summary of projected cash and runway fan chart (10th, 50th, and 90th percentiles).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RunwayProjection {
    pub cash_now: Money,
    pub credit_available: Money,
    pub gross_monthly_burn: Money,
    pub net_monthly_burn: Money,
    pub runway_days_p10: i64,
    pub runway_days_p50: i64,
    pub runway_days_p90: i64,
    pub bankrupt: bool,
}

impl RunwayProjection {
    pub fn compute(
        cash_now: Money,
        credit_available: Money,
        gross_monthly_burn: Money,
        monthly_revenue: Money,
    ) -> Self {
        let liquid = Money(cash_now.0 + credit_available.0);
        let net_burn = Money(gross_monthly_burn.0 - monthly_revenue.0);
        let p50 = reference::reference_runway_days(liquid, net_burn);
        let p10 = reference::reference_runway_days(
            liquid,
            Money(gross_monthly_burn.0 - (monthly_revenue.0 * 65) / 100),
        );
        let p90 = reference::reference_runway_days(
            liquid,
            Money(gross_monthly_burn.0 - (monthly_revenue.0 * 135) / 100),
        );
        Self {
            cash_now,
            credit_available,
            gross_monthly_burn,
            net_monthly_burn: net_burn,
            runway_days_p10: p10.min(p50),
            runway_days_p50: p50,
            runway_days_p90: p90.max(p50),
            bankrupt: liquid.0 < 0,
        }
    }
}
