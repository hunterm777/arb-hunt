//! Exact decimal helpers. Every amount in arbcalc is a `rust_decimal::Decimal`; nothing
//! passes through a binary float.

use std::str::FromStr;

use serde_json::{Number, Value};
use venue_specs::Decimal;

/// Largest multiple of `step` that is `<= x`.
pub fn floor_to(x: Decimal, step: Decimal) -> Decimal {
    (x / step).floor() * step
}

/// A decimal from JSON: a number (read from its exact text) or a numeric string.
pub fn from_json(v: &Value) -> Option<Decimal> {
    let text = match v {
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.trim().to_owned(),
        _ => return None,
    };
    Decimal::from_str(&text)
        .or_else(|_| Decimal::from_scientific(&text))
        .ok()
}

/// A decimal as a JSON number carrying its exact text, trailing zeros dropped.
pub fn to_json(d: Decimal) -> Value {
    let text = d.normalize().to_string();
    Value::Number(Number::from_str(&text).expect("a decimal's text is a JSON number"))
}

/// Dollars for a sentence: the exact amount, with at least two decimals.
pub fn dollars(d: Decimal) -> String {
    let d = d.normalize();
    let d = if d.scale() < 2 {
        let mut x = d;
        x.rescale(2);
        x
    } else {
        d
    };
    format!("${d}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn dec(s: &str) -> Decimal {
        Decimal::from_str(s).unwrap()
    }

    #[test]
    fn floors_to_a_step() {
        assert_eq!(floor_to(dec("0.0167"), dec("0.01")), dec("0.01"));
        assert_eq!(floor_to(dec("-0.004"), dec("0.01")), dec("-0.01"));
    }

    #[test]
    fn reads_numbers_and_strings_exactly() {
        assert_eq!(from_json(&json!(0.41)), Some(dec("0.41")));
        assert_eq!(from_json(&json!("1e-5")), Some(dec("0.00001")));
        assert_eq!(from_json(&json!(true)), None);
        assert_eq!(from_json(&json!("x")), None);
    }

    #[test]
    fn writes_exact_numbers() {
        assert_eq!(to_json(dec("1.7000")).to_string(), "1.7");
        assert_eq!(to_json(dec("-0.9375")).to_string(), "-0.9375");
    }

    #[test]
    fn formats_dollars() {
        assert_eq!(dollars(dec("1.7")), "$1.70");
        assert_eq!(dollars(dec("2.0525")), "$2.0525");
        assert_eq!(dollars(dec("100")), "$100.00");
    }
}
