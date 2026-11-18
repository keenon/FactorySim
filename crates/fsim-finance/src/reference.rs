//! Reference oracle for `fsim-finance` equipment loan underwriting, monthly amortization, and runway.

use fsim_core::Money;

/// Reference monthly payment in tenth-cents for a 36-month equipment loan at `apr_bps` basis points.
pub fn reference_monthly_loan_payment(principal: Money, apr_bps: u32, term_months: u32) -> Money {
    let m = term_months.max(1) as i64;
    let total_interest = (principal.0 * (apr_bps as i64) * m) / (12 * 10_000);
    Money((principal.0 + total_interest) / m)
}

/// Reference runway in days given current cash + available credit and net monthly burn.
pub fn reference_runway_days(liquid_funds: Money, net_monthly_burn: Money) -> i64 {
    if liquid_funds.0 <= 0 {
        return 0;
    }
    if net_monthly_burn.0 <= 0 {
        return 3_650; // Profitable / cash-flow positive (capped at 10 years for display)
    }
    ((liquid_funds.0 * 30) / net_monthly_burn.0).clamp(0, 3_650)
}
