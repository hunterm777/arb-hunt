//! Profit if the event happens and if it does not. Fees charged at settlement depend on
//! which leg pays, so the two can differ; the guaranteed profit is the lower one.

use venue_specs::Decimal;

use crate::error::Result;
use crate::fees::LegRules;
use crate::leg::{Leg, Side};
use crate::pairing::LegTotals;

/// One outcome's result for N pairs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Outcome {
    /// Total paid out across both legs.
    pub payout: Decimal,
    /// Fees charged at settlement across both legs.
    pub settlement_fees: [Decimal; 2],
    /// Net profit after every cost.
    pub profit: Decimal,
}

/// Payout per contract on each leg, for each outcome: `[happens, does not happen]`.
pub fn payouts(legs: &[Leg; 2], rules: &[LegRules; 2]) -> Result<[[Decimal; 2]; 2]> {
    let mut out = [[Decimal::ZERO; 2]; 2];
    for (k, (leg, r)) in legs.iter().zip(rules).enumerate() {
        let yes = leg.side == Side::Yes;
        out[k] = [r.payout(yes, true)?, r.payout(yes, false)?];
    }
    Ok(out)
}

/// The result of one outcome (`o` = 0 if the event happens, 1 if not).
pub fn outcome(
    o: usize,
    totals: &[LegTotals; 2],
    rules: &[LegRules; 2],
    pay: &[[Decimal; 2]; 2],
    fixed: Decimal,
) -> Result<Outcome> {
    let mut payout = Decimal::ZERO;
    let mut settle = [Decimal::ZERO; 2];
    let mut cost = fixed;
    for k in 0..2 {
        let px = pay[k][o];
        let c = totals[k].contracts;
        payout += px * c;
        settle[k] = rules[k].settlement_fee(c, px)?;
        cost += totals[k].cost + totals[k].fees + settle[k];
    }
    Ok(Outcome {
        payout,
        settlement_fees: settle,
        profit: payout - cost,
    })
}
