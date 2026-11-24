//! Input errors: each is one sentence that says what to fix.

use arbcalc_core::api::check_json;
use serde_json::{json, Value};

fn base() -> Value {
    json!({
        "legs": [
            {"venue": "kalshi", "side": "yes", "price": 0.41, "size": 100, "model_fee": "0.07 * C * P * (1 - P)"},
            {"venue": "polymarket", "side": "no", "category": "economics", "price": 0.55, "size": 100}
        ],
        "payout_days": 45, "rules_date": "2026-11-24"
    })
}

fn err(edit: impl FnOnce(&mut Value), want: &str) -> String {
    let mut v = base();
    edit(&mut v);
    check_json(&v.to_string())
        .err()
        .unwrap_or_else(|| panic!("no error for {want:?}"))
}

#[test]
fn the_base_case_runs() {
    assert!(check_json(&base().to_string()).is_ok());
}

type Edit = Box<dyn FnOnce(&mut Value)>;

#[test]
fn input_errors_say_what_to_fix() {
    let cases: Vec<(Edit, &str)> = vec![
        (
            Box::new(|v| v["legs"][1]["side"] = json!("yes")),
            "same side",
        ),
        (
            Box::new(|v| v["legs"][0]["venue"] = json!("betfair")),
            "unknown venue",
        ),
        (
            Box::new(|v| v["legs"][0]["side"] = json!("up")),
            "unknown side",
        ),
        (
            Box::new(|v| v["legs"][0]["role"] = json!("whale")),
            "unknown role",
        ),
        (
            Box::new(|v| v["legs"][0]["price"] = json!(1)),
            "between 0 and 1",
        ),
        (Box::new(|v| v["legs"][0]["size"] = json!(0)), "above 0"),
        (
            Box::new(|v| v["legs"][0]["size"] = json!(10.005)),
            "steps of 0.01",
        ),
        (
            Box::new(|v| v["legs"][1]["category"] = json!("chess")),
            "unknown fee category",
        ),
        (
            Box::new(|v| v["legs"][0]["category"] = json!("sports")),
            "no fee categories",
        ),
        (
            Box::new(|v| v["legs"][1]["model_fee"] = json!("0.01 * C")),
            "does not apply",
        ),
        (
            Box::new(|v| v["legs"][0]["model_fee"] = json!("0.07 * C * X")),
            "may use C and P",
        ),
        (
            Box::new(|v| v["legs"][0]["model_fee"] = json!("0.07 * (C")),
            "model fee formula",
        ),
        (
            Box::new(|v| v["legs"][0]["payout_date"] = json!("2027-01-01")),
            "not both",
        ),
        (
            Box::new(|v| {
                v.as_object_mut().unwrap().remove("payout_days");
            }),
            "payout date",
        ),
        (
            Box::new(|v| v["payout_days"] = json!(0)),
            "whole number of days",
        ),
        (
            Box::new(|v| v["payout_days"] = json!(1.5)),
            "whole number of days",
        ),
        (
            Box::new(|v| v["rules_date"] = json!("2026-13-01")),
            "not a date",
        ),
        (
            Box::new(|v| {
                v.as_object_mut().unwrap().remove("rules_date");
            }),
            "rules date",
        ),
        (
            Box::new(|v| v["fixed_costs"] = json!(-1)),
            "cannot be negative",
        ),
        (
            Box::new(|v| v["fixed_costs"] = json!("two")),
            "must be a number",
        ),
        (Box::new(|v| v["legs"] = json!([])), "exactly two legs"),
        (
            Box::new(|v| v["legs"][0]["size"] = json!(0.5)),
            "whole contract",
        ),
        (
            Box::new(|v| v["legs"][0]["levels"] = json!([[0.4, 1]])),
            "not both",
        ),
        (
            Box::new(|v| {
                v["legs"][0].as_object_mut().unwrap().remove("price");
            }),
            "price and size",
        ),
        (
            Box::new(|v| {
                v["legs"][0]["size"] = json!(2_000_000);
                v["legs"][1]["size"] = json!(2_000_000);
            }),
            "enter less depth",
        ),
    ];
    for (edit, want) in cases {
        let e = err(edit, want);
        assert!(e.contains(want), "{want:?} not in {e:?}");
    }
}

#[test]
fn malformed_json_is_an_error() {
    assert!(check_json("{").unwrap_err().contains("not JSON"));
    assert!(check_json("[]").unwrap_err().contains("JSON object"));
}

/// Levels are walked cheapest first however they are entered.
#[test]
fn levels_are_walked_cheapest_first() {
    let mut a = base();
    a["legs"][0] = json!({"venue": "kalshi", "side": "yes", "levels": [[0.42, 50], [0.41, 50]], "model_fee": "0.07 * C * P * (1 - P)"});
    let mut b = a.clone();
    b["legs"][0]["levels"] = json!([{"price": 0.41, "size": 50}, {"price": 0.42, "size": 50}]);
    let ra: Value = serde_json::from_str(&check_json(&a.to_string()).unwrap()).unwrap();
    let rb: Value = serde_json::from_str(&check_json(&b.to_string()).unwrap()).unwrap();
    assert_eq!(ra["sizing"], rb["sizing"]);
    assert_eq!(ra["sizing"]["average_price"][0], json!(0.415));
    assert_eq!(ra["legs"][0]["fills"], json!(2));
}

#[test]
fn meta_lists_categories_and_unverified_fees() {
    let m: Value = serde_json::from_str(&arbcalc_core::api::meta()).unwrap();
    assert!(m["venues"]["polymarket"]["categories"]
        .as_array()
        .unwrap()
        .contains(&json!("economics")));
    assert!(m["venues"]["kalshi"]["model_fee_unverified"]["taker"].is_string());
    assert!(m["venues"]["hyperliquid"]["model_fee_unverified"]["taker"].is_null());
}
