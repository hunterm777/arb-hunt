//! Walking both books together. Each level the walk reaches is one fill, so the first
//! N contracts of a leg are some whole levels plus part of the next one. Totals for the
//! whole levels are carried forward, so pairing N contracts costs one fee evaluation per
//! leg for the partial fill.

use venue_specs::Decimal;

use crate::error::Result;
use crate::fees::{FillFee, LegRules};
use crate::leg::Level;

/// What the first N contracts of one leg cost.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LegTotals {
    /// Contracts bought.
    pub contracts: Decimal,
    /// Sum of price x contracts.
    pub cost: Decimal,
    /// Sum of model fees (before rounding).
    pub model: Decimal,
    /// Entry fees paid: fees charged less per-order rebates.
    pub fees: Decimal,
    /// Fills used.
    pub fills: usize,
    /// Price of the last contract bought.
    pub last_price: Decimal,
    /// Rounding overpayment carried in the per-order accumulator.
    pub carry: Decimal,
}

impl LegTotals {
    /// The fee rounding cost: entry fees paid less model fees.
    pub fn rounding(&self) -> Decimal {
        self.fees - self.model
    }

    /// Average price paid per contract.
    pub fn average_price(&self) -> Decimal {
        if self.contracts.is_zero() {
            Decimal::ZERO
        } else {
            self.cost / self.contracts
        }
    }

    fn add(&self, rules: &LegRules, level: Level, size: Decimal, fee: FillFee) -> LegTotals {
        let (net, carry) = match rules.accumulator() {
            Some(acc) => acc.apply(fee, self.carry),
            None => (fee.charged, self.carry),
        };
        LegTotals {
            contracts: self.contracts + size,
            cost: self.cost + level.price * size,
            model: self.model + fee.model,
            fees: self.fees + net,
            fills: self.fills + 1,
            last_price: level.price,
            carry,
        }
    }
}

/// One leg's book, walked in order of N.
pub struct Walker<'a> {
    rules: &'a LegRules,
    book: Vec<Level>,
    /// Totals after every whole level before `at`.
    done: LegTotals,
    at: usize,
}

impl<'a> Walker<'a> {
    /// A walker over `book`, cheapest level first.
    pub fn new(rules: &'a LegRules, book: Vec<Level>) -> Walker<'a> {
        Walker {
            rules,
            book,
            done: LegTotals::default(),
            at: 0,
        }
    }

    /// Totals for the first `n` contracts. Calls must not decrease `n`.
    pub fn totals(&mut self, n: Decimal) -> Result<LegTotals> {
        loop {
            let level = self.book[self.at];
            let before = self.done.contracts;
            let take = n - before;
            if take <= level.size || self.at + 1 == self.book.len() {
                let fee = self.rules.fill(take, level.price)?;
                return Ok(self.done.add(self.rules, level, take, fee));
            }
            let fee = self.rules.fill(level.size, level.price)?;
            self.done = self.done.add(self.rules, level, level.size, fee);
            self.at += 1;
        }
    }
}
