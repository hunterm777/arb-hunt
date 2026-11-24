//! One leg of the arb: a venue, a side, the price levels to buy from, and the fee inputs.

use venue_specs::Decimal;

use crate::error::{Error, Result};

/// The venues arbcalc prices, each mapped to its venue-specs spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Venue {
    /// Kalshi event contracts (`kalshi/binary`).
    Kalshi,
    /// Polymarket prediction markets (`polymarket/binary`).
    Polymarket,
    /// Hyperliquid HIP-4 outcome markets (`hyperliquid/outcome`).
    Hyperliquid,
}

impl Venue {
    /// Parse `kalshi`, `polymarket` or `hyperliquid`.
    pub fn parse(s: &str) -> Result<Venue> {
        match s.trim().to_ascii_lowercase().as_str() {
            "kalshi" => Ok(Venue::Kalshi),
            "polymarket" => Ok(Venue::Polymarket),
            "hyperliquid" => Ok(Venue::Hyperliquid),
            _ => Err(Error::input(format!(
                "unknown venue {s:?}: use kalshi, polymarket or hyperliquid"
            ))),
        }
    }

    /// The venue key in venue-specs.
    pub fn key(self) -> &'static str {
        match self {
            Venue::Kalshi => "kalshi",
            Venue::Polymarket => "polymarket",
            Venue::Hyperliquid => "hyperliquid",
        }
    }

    /// The product in venue-specs.
    pub fn product(self) -> &'static str {
        match self {
            Venue::Kalshi | Venue::Polymarket => "binary",
            Venue::Hyperliquid => "outcome",
        }
    }

    /// The name shown in reports.
    pub fn label(self) -> &'static str {
        match self {
            Venue::Kalshi => "Kalshi",
            Venue::Polymarket => "Polymarket",
            Venue::Hyperliquid => "Hyperliquid HIP-4",
        }
    }
}

/// Which side of the binary the leg buys.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Side {
    /// Pays out if the event happens.
    Yes,
    /// Pays out if the event does not happen.
    No,
}

impl Side {
    /// Parse `yes` or `no`.
    pub fn parse(s: &str) -> Result<Side> {
        match s.trim().to_ascii_lowercase().as_str() {
            "yes" => Ok(Side::Yes),
            "no" => Ok(Side::No),
            _ => Err(Error::input(format!("unknown side {s:?}: use yes or no"))),
        }
    }

    /// `yes` or `no`.
    pub fn key(self) -> &'static str {
        match self {
            Side::Yes => "yes",
            Side::No => "no",
        }
    }
}

/// Maker or taker: selects `fees.maker` or `fees.taker` in venue-specs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Role {
    /// The order takes liquidity (the default).
    #[default]
    Taker,
    /// The order rests on the book and is filled.
    Maker,
}

impl Role {
    /// Parse `taker` or `maker`.
    pub fn parse(s: &str) -> Result<Role> {
        match s.trim().to_ascii_lowercase().as_str() {
            "taker" => Ok(Role::Taker),
            "maker" => Ok(Role::Maker),
            _ => Err(Error::input(format!(
                "unknown role {s:?}: use taker or maker"
            ))),
        }
    }

    /// `taker` or `maker`.
    pub fn key(self) -> &'static str {
        match self {
            Role::Taker => "taker",
            Role::Maker => "maker",
        }
    }
}

/// One price level: contracts available at a price. Each level the walk reaches is one fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Level {
    /// Price paid per contract, in dollars, strictly between 0 and 1.
    pub price: Decimal,
    /// Contracts available at that price.
    pub size: Decimal,
}

/// One leg as entered.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Leg {
    /// The venue.
    pub venue: Venue,
    /// The side bought.
    pub side: Side,
    /// Price levels; top of book is one level. Walked cheapest first.
    pub levels: Vec<Level>,
    /// Fee category (`fees.categories.<key>` in venue-specs), where the venue has one.
    pub category: Option<String>,
    /// Maker or taker.
    pub role: Role,
    /// The leg's model fee formula, used only where venue-specs marks the model fee
    /// unverified. It may use `C` (contracts) and `P` (price).
    pub model_fee: Option<String>,
    /// Expected payout date, `YYYY-MM-DD`.
    pub payout_date: Option<String>,
}

impl Leg {
    /// The levels sorted cheapest first; equal prices keep the order entered.
    pub fn book(&self) -> Vec<Level> {
        let mut b = self.levels.clone();
        b.sort_by_key(|l| l.price);
        b
    }

    /// Contracts available across all levels.
    pub fn depth(&self) -> Decimal {
        self.levels.iter().map(|l| l.size).sum()
    }

    pub(crate) fn check_levels(&self, name: &str) -> Result<()> {
        if self.levels.is_empty() {
            return Err(Error::input(format!(
                "{name}: enter a price and size, or price levels"
            )));
        }
        for l in &self.levels {
            if l.price <= Decimal::ZERO || l.price >= Decimal::ONE {
                return Err(Error::input(format!(
                    "{name}: price {} must be between 0 and 1",
                    l.price.normalize()
                )));
            }
            if l.size <= Decimal::ZERO {
                return Err(Error::input(format!(
                    "{name}: size {} must be above 0",
                    l.size.normalize()
                )));
            }
        }
        Ok(())
    }
}
