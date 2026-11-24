//! Capital, return on capital and simple annualised return.

use venue_specs::Decimal;

use crate::pairing::LegTotals;

/// Decimal places kept for ROC and APR.
pub const RATIO_DP: u32 = 12;

/// Returns for a trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Returns {
    /// Cost of both legs plus entry fees plus fixed costs.
    pub capital: Decimal,
    /// Guaranteed profit over capital.
    pub roc: Decimal,
    /// Days from the rules date to the later payout date.
    pub days: i64,
    /// ROC x 365 / days (simple, not compounded).
    pub apr: Decimal,
}

/// Returns on `profit` for the legs' totals, `fixed` costs and `days` to payout.
pub fn returns(profit: Decimal, totals: &[LegTotals; 2], fixed: Decimal, days: i64) -> Returns {
    let capital = totals.iter().map(|t| t.cost + t.fees).sum::<Decimal>() + fixed;
    let roc = if capital.is_zero() {
        Decimal::ZERO
    } else {
        profit / capital
    };
    let apr = roc * Decimal::from(365) / Decimal::from(days);
    Returns {
        capital,
        roc: round(roc),
        days,
        apr: round(apr),
    }
}

/// Round half away from zero to [`RATIO_DP`] places.
pub fn round(x: Decimal) -> Decimal {
    x.round_dp_with_strategy(
        RATIO_DP,
        rust_decimal::RoundingStrategy::MidpointAwayFromZero,
    )
}
