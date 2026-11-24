//! arbcalc: does a cross-venue prediction-market arb still make money after each venue's
//! fees, rounding and capital lock-up?
//!
//! An arb buys YES on one venue and NO on another for the same event; exactly one leg
//! pays out. Every fee, rounding step and payout rule comes from
//! [venue-specs](https://github.com/hunterm777/venue-specs), never from this crate.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod api;
pub mod dates;
pub mod error;
pub mod fees;
pub mod leg;
pub mod num;
pub mod outcomes;
pub mod pairing;
pub mod report;
pub mod returns;

pub use error::{Error, Result};
pub use leg::{Leg, Level, Role, Side, Venue};
pub use report::{check_with, Input, Report};
pub use venue_specs::Decimal;

/// Run a check against the venue-specs rules compiled in.
pub fn check(input: &Input) -> Result<Report> {
    check_with(&fees::Embedded, input)
}
