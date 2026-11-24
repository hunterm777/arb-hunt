//! JSON in, JSON out: the one entry point the page (WASM) and Python use, so both run
//! exactly the same code as native Rust.
//!
//! Input:
//!
//! ```json
//! {
//!   "event": "Fed cuts in December",
//!   "legs": [
//!     {"venue": "kalshi", "side": "yes", "levels": [[0.41, 300], [0.42, 500]],
//!      "model_fee": "0.07 * C * P * (1 - P)"},
//!     {"venue": "polymarket", "side": "no", "category": "economics",
//!      "levels": [[0.55, 200], [0.56, 800]]}
//!   ],
//!   "fixed_costs": 2.00,
//!   "payout_days": 45,
//!   "rules_date": "2026-11-24"
//! }
//! ```
//!
//! A leg takes `levels` (`[price, size]` pairs) or `price` and `size`, plus optional
//! `category`, `role` (`taker`, the default, or `maker`), `model_fee` and `payout_date`.
//! Give `payout_days` or a `payout_date` on each leg.

use serde_json::{Map, Value};
use venue_specs::Decimal;

use crate::error::{Error, Result};
use crate::fees::{Embedded, Rules};
use crate::leg::{Leg, Level, Role, Side, Venue};
use crate::num::from_json;
use crate::report::{check_with, Input};

fn field<'a>(m: &'a Map<String, Value>, k: &str) -> Option<&'a Value> {
    m.get(k).filter(|v| !v.is_null())
}

fn text(m: &Map<String, Value>, k: &str, at: &str) -> Result<Option<String>> {
    match field(m, k) {
        None => Ok(None),
        Some(Value::String(s)) if s.trim().is_empty() => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.trim().to_owned())),
        Some(_) => Err(Error::input(format!("{at}{k} must be text"))),
    }
}

fn number(v: &Value, what: &str) -> Result<Decimal> {
    from_json(v).ok_or_else(|| Error::input(format!("{what} must be a number")))
}

fn level(v: &Value, at: &str) -> Result<Level> {
    match v {
        Value::Array(a) if a.len() == 2 => Ok(Level {
            price: number(&a[0], &format!("{at}price"))?,
            size: number(&a[1], &format!("{at}size"))?,
        }),
        Value::Object(o) => Ok(Level {
            price: number(
                o.get("price").unwrap_or(&Value::Null),
                &format!("{at}price"),
            )?,
            size: number(o.get("size").unwrap_or(&Value::Null), &format!("{at}size"))?,
        }),
        _ => Err(Error::input(format!("{at}each level is [price, size]"))),
    }
}

fn leg(v: &Value, name: &str) -> Result<Leg> {
    let at = format!("{name}: ");
    let m = v
        .as_object()
        .ok_or_else(|| Error::input(format!("{name} must be an object")))?;
    let venue = Venue::parse(&text(m, "venue", &at)?.unwrap_or_default())?;
    let side = Side::parse(&text(m, "side", &at)?.unwrap_or_default())?;
    let role = match text(m, "role", &at)? {
        Some(r) => Role::parse(&r)?,
        None => Role::Taker,
    };
    let levels = match (field(m, "levels"), field(m, "price"), field(m, "size")) {
        (Some(Value::Array(ls)), None, None) => ls
            .iter()
            .map(|l| level(l, &at))
            .collect::<Result<Vec<_>>>()?,
        (Some(_), None, None) => return Err(Error::input(format!("{at}levels must be a list"))),
        (None, Some(p), Some(s)) => vec![Level {
            price: number(p, &format!("{at}price"))?,
            size: number(s, &format!("{at}size"))?,
        }],
        (None, _, _) => {
            return Err(Error::input(format!(
                "{at}enter a price and size, or price levels"
            )))
        }
        _ => {
            return Err(Error::input(format!(
                "{at}give levels or price and size, not both"
            )))
        }
    };
    Ok(Leg {
        venue,
        side,
        levels,
        category: text(m, "category", &at)?,
        role,
        model_fee: text(m, "model_fee", &at)?,
        payout_date: text(m, "payout_date", &at)?,
    })
}

/// Parse an input document.
pub fn parse_input(json: &str) -> Result<Input> {
    let v: Value =
        serde_json::from_str(json).map_err(|e| Error::input(format!("input is not JSON: {e}")))?;
    let m = v
        .as_object()
        .ok_or_else(|| Error::input("input must be a JSON object"))?;
    let legs = match field(m, "legs") {
        Some(Value::Array(a)) if a.len() == 2 => [leg(&a[0], "Leg A")?, leg(&a[1], "Leg B")?],
        _ => return Err(Error::input("enter exactly two legs")),
    };
    let fixed_costs = match field(m, "fixed_costs") {
        None => Decimal::ZERO,
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| number(x, "each fixed cost"))
            .sum::<Result<Decimal>>()?,
        Some(x) => number(x, "fixed_costs")?,
    };
    let payout_days = match field(m, "payout_days") {
        None => None,
        Some(x) => {
            let d = number(x, "payout_days")?;
            if !d.fract().is_zero() || d <= Decimal::ZERO || d > Decimal::from(36_500) {
                return Err(Error::input(
                    "payout_days must be a whole number of days above 0",
                ));
            }
            Some(i64::try_from(d.mantissa()).unwrap_or(0) / 10i64.pow(d.normalize().scale()))
        }
    };
    let rules_date =
        text(m, "rules_date", "")?.ok_or_else(|| Error::input("enter the rules date"))?;
    Ok(Input {
        event: text(m, "event", "")?.unwrap_or_default(),
        legs,
        fixed_costs,
        payout_days,
        rules_date,
    })
}

/// Run a check against `rules` and return the report as JSON text.
pub fn check_json_with(rules: &dyn Rules, json: &str) -> std::result::Result<String, String> {
    let input = parse_input(json).map_err(|e| e.to_string())?;
    let report = check_with(rules, &input).map_err(|e| e.to_string())?;
    Ok(report.to_json().to_string())
}

/// Run a check against the embedded venue-specs and return the report as JSON text.
/// Errors are one sentence that says what to fix.
pub fn check_json(json: &str) -> std::result::Result<String, String> {
    check_json_with(&Embedded, json)
}

/// The venue-specs data version compiled in, and the fee categories and
/// model-fee status per venue, for the page's inputs.
pub fn meta() -> String {
    let mut venues = Map::new();
    for v in [Venue::Kalshi, Venue::Polymarket, Venue::Hyperliquid] {
        let spec = venue_specs::spec(v.key(), v.product()).ok();
        let cats: Vec<String> = match spec.and_then(|s| s.get("fees.categories")) {
            Some(venue_specs::Node::Map(rows)) => rows.keys().cloned().collect(),
            _ => vec![],
        };
        let unverified =
            |role: &str| match spec.and_then(|s| s.get(&format!("fees.{role}.model_fee"))) {
                Some(venue_specs::Node::Unverified(u)) => Value::String(u.reason.clone()),
                _ => Value::Null,
            };
        venues.insert(
            v.key().to_owned(),
            serde_json::json!({
                "label": v.label(),
                "categories": cats,
                "model_fee_unverified": { "taker": unverified("taker"), "maker": unverified("maker") },
            }),
        );
    }
    serde_json::json!({ "venue_specs": venue_specs::version(), "venues": venues }).to_string()
}
