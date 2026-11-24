//! The arb math under rules that venue-specs does not hold today: a test bundle derived
//! from the embedded one, with chosen entries changed. This checks the PRD's own worked
//! numbers, settlement fees that differ by outcome, rules that change before payout, and
//! the per-order rounding accumulator.

use std::str::FromStr;

use arbcalc_core::api::parse_input;
use arbcalc_core::fees::Rules;
use arbcalc_core::{check_with, Decimal, Error, Report};
use serde_json::{json, Value};
use venue_specs::{Bundle, Spec};

/// The embedded bundle with edits; `later` applies from `switch` (a date) onwards.
struct TestRules {
    now: Bundle,
    later: Option<(String, Bundle)>,
}

fn edit(edits: &[(&str, &str, &str, Value)]) -> Bundle {
    let mut b: Value = serde_json::from_str(venue_specs::BUNDLE_JSON).unwrap();
    for (venue, product, path, leaf) in edits {
        let spec = b["specs"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|s| s["venue"] == *venue && s["product"] == *product)
            .unwrap();
        let mut node = spec;
        let parts: Vec<&str> = path.split('.').collect();
        for p in &parts[..parts.len() - 1] {
            node = node.as_object_mut().unwrap().entry(*p).or_insert(json!({}));
        }
        node[parts[parts.len() - 1]] = leaf.clone();
    }
    Bundle::from_json_str(&b.to_string()).unwrap()
}

fn sourced_formula(f: &str) -> Value {
    json!({ "formula": f, "source": "test", "as_of": "2026-11-24" })
}

fn sourced_value(v: Value) -> Value {
    json!({ "value": v, "source": "test", "as_of": "2026-11-24" })
}

impl Rules for TestRules {
    fn spec_at(&self, venue: &str, product: &str, date: &str) -> arbcalc_core::Result<Spec> {
        let b = match &self.later {
            Some((d, b)) if date >= d.as_str() => b,
            _ => &self.now,
        };
        b.spec(venue, product)
            .cloned()
            .map_err(|e| Error::Rules(e.to_string()))
    }

    fn version(&self) -> String {
        "test".into()
    }
}

fn rules(edits: &[(&str, &str, &str, Value)]) -> TestRules {
    TestRules {
        now: edit(edits),
        later: None,
    }
}

fn run(r: &TestRules, input: Value) -> arbcalc_core::Result<Report> {
    check_with(r, &parse_input(&input.to_string())?)
}

fn d(s: &str) -> Decimal {
    Decimal::from_str(s).unwrap()
}

const K: &str = "0.07 * C * P * (1 - P)";

fn prd_case(fixed: f64) -> Value {
    json!({
        "legs": [
            {"venue": "kalshi", "side": "yes", "price": 0.41, "size": 100, "model_fee": K},
            {"venue": "polymarket", "side": "no", "category": "economics", "price": 0.55, "size": 100}
        ],
        "fixed_costs": fixed, "payout_days": 45, "rules_date": "2026-11-24"
    })
}

/// The PRD's acceptance numbers hold when the Polymarket economics rate is the PRD's 0.01.
#[test]
fn prd_numbers_at_the_prds_rate() {
    let r = rules(&[(
        "polymarket",
        "binary",
        "fees.categories.economics.rate",
        sourced_value(json!(0.01)),
    )]);
    let a = run(&r, prd_case(0.0)).unwrap();
    assert_eq!(a.legs[0].fees, d("1.70"));
    assert_eq!(a.legs[1].fees, d("0.2475"));
    assert_eq!(a.guaranteed_profit, d("2.0525"));
    assert_eq!(a.verdict, "Profitable: $2.0525 guaranteed on 100 pairs");
    // ROC 2.10%, APR 17.0% at 45 days.
    assert_eq!(a.returns.roc.round_dp(4), d("0.0210"));
    assert_eq!(a.returns.apr.round_dp(3), d("0.170"));
    let b = run(&r, prd_case(2.0)).unwrap();
    assert_eq!(b.guaranteed_profit, d("0.0525"));
}

fn hip4_case(side: &str, payout_days: i64) -> Value {
    json!({
        "legs": [
            {"venue": "hyperliquid", "side": side, "price": 0.41, "size": 100},
            {"venue": "polymarket", "side": if side == "yes" { "no" } else { "yes" },
             "category": "geopolitics", "price": 0.55, "size": 100}
        ],
        "payout_days": payout_days, "rules_date": "2026-11-24"
    })
}

/// A HIP-4 fee lands at settlement on the winning leg only, so the outcomes differ and the
/// guaranteed profit is the lower one; the opening fill pays nothing.
#[test]
fn hip4_settlement_fee_differs_by_outcome() {
    let r = rules(&[(
        "hyperliquid",
        "outcome",
        "fees.taker.fee",
        sourced_formula("0.001 * N"),
    )]);
    let a = run(&r, hip4_case("yes", 45)).unwrap();
    assert_eq!(a.legs[0].fees, Decimal::ZERO);
    assert_eq!(a.outcomes[0].settlement_fees[0], d("0.1"));
    assert_eq!(a.outcomes[1].settlement_fees[0], Decimal::ZERO);
    assert_eq!(a.outcomes[0].profit, d("3.9"));
    assert_eq!(a.outcomes[1].profit, d("4"));
    assert_eq!(a.guaranteed_profit, d("3.9"));
    let label = &a.breakdown[a.breakdown.len() - 3];
    assert_eq!(label.label, "Hyperliquid HIP-4 settlement fee");
    assert_eq!(label.amount, d("-0.1"));
    // Holding NO on HIP-4, the fee lands if the event does not happen.
    let b = run(&r, hip4_case("no", 45)).unwrap();
    assert_eq!(b.outcomes[0].profit, d("4"));
    assert_eq!(b.outcomes[1].profit, d("3.9"));
}

/// HIP-4 fees are charged under the rules in force at settlement, not at entry.
#[test]
fn settlement_fee_uses_payout_date_rules() {
    let r = TestRules {
        now: edit(&[]),
        later: Some((
            "2026-12-15".into(),
            edit(&[(
                "hyperliquid",
                "outcome",
                "fees.taker.fee",
                sourced_formula("0.002 * N"),
            )]),
        )),
    };
    let before = run(&r, hip4_case("yes", 10)).unwrap();
    assert_eq!(before.guaranteed_profit, d("4"));
    let after = run(&r, hip4_case("yes", 45)).unwrap();
    assert_eq!(after.outcomes[0].settlement_fees[0], d("0.2"));
    assert_eq!(after.guaranteed_profit, d("3.8"));
    assert_eq!(
        after.provenance[0]["settlement_fee"]["rules_date"],
        "2027-01-08"
    );
}

/// Kalshi's per-order accumulator: three fills that each overpay $0.004 get $0.01 back on
/// the third, and $0.002 stays carried.
#[test]
fn accumulator_rebates_whole_cents() {
    let r = rules(&[]);
    let input = json!({
        "legs": [
            {"venue": "kalshi", "side": "yes", "levels": [[0.5, 1], [0.5, 1], [0.5, 1]], "model_fee": "0.006"},
            {"venue": "polymarket", "side": "no", "category": "geopolitics", "price": 0.3, "size": 3}
        ],
        "payout_days": 1, "rules_date": "2026-11-24"
    });
    let a = run(&r, input).unwrap();
    assert_eq!(a.pairs, 3);
    let k = a.legs[0];
    assert_eq!(k.fills, 3);
    assert_eq!(k.model, d("0.018"));
    assert_eq!(k.fees, d("0.02"));
    assert_eq!(k.carry, d("0.002"));
    assert_eq!(k.rounding(), d("0.002"));
}

/// A model fee formula fills an unverified entry only; once venue-specs has a sourced
/// value, the sourced value wins and the report says so.
#[test]
fn sourced_model_fee_beats_your_formula() {
    let r = rules(&[(
        "kalshi",
        "binary",
        "fees.taker.model_fee",
        sourced_formula(K),
    )]);
    let mut input = prd_case(0.0);
    input["legs"][0]["model_fee"] = json!("0.5 * C");
    let a = run(&r, input.clone()).unwrap();
    assert_eq!(a.legs[0].fees, d("1.70"));
    assert!(a.notes[0].contains("now has a sourced value"));
    input["legs"][0]
        .as_object_mut()
        .unwrap()
        .remove("model_fee");
    assert_eq!(run(&r, input).unwrap().legs[0].fees, d("1.70"));
}

/// A rule arbcalc does not model is an error, never a guess.
#[test]
fn unknown_rules_are_errors() {
    let r = rules(&[(
        "hyperliquid",
        "outcome",
        "fees.when_charged",
        sourced_value(json!("monthly")),
    )]);
    let e = run(&r, hip4_case("yes", 45)).unwrap_err().to_string();
    assert!(e.contains("does not model"), "{e}");
    let r = rules(&[(
        "kalshi",
        "binary",
        "fees.rounding.accumulator",
        sourced_value(json!("per_day")),
    )]);
    let e = run(&r, prd_case(0.0)).unwrap_err().to_string();
    assert!(e.contains("does not model"), "{e}");
}
