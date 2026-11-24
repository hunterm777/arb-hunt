//! The shared fixture suite: every case in fixtures/cases must give exactly the text in
//! fixtures/expected, byte for byte. The WASM and Python builds run the same files.
//! Regenerate with `BLESS=1 cargo test -p arbcalc-core --test fixtures` and review the diff.

use std::fs;
use std::path::PathBuf;

use arbcalc_core::api::check_json;
use serde_json::{json, Value};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures")
}

fn run(input: &str) -> String {
    match check_json(input) {
        Ok(out) => out,
        Err(e) => json!({ "error": e }).to_string(),
    }
}

#[test]
fn fixtures_match_byte_for_byte() {
    let bless = std::env::var_os("BLESS").is_some();
    let mut names: Vec<_> = fs::read_dir(root().join("cases"))
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .filter(|n| n.ends_with(".json"))
        .collect();
    names.sort();
    assert!(names.len() >= 12, "fixture suite is missing cases");
    let mut failed = vec![];
    for name in &names {
        let got = run(&fs::read_to_string(root().join("cases").join(name)).unwrap());
        let path = root().join("expected").join(name);
        if bless {
            fs::write(&path, format!("{got}\n")).unwrap();
            continue;
        }
        let want = fs::read_to_string(&path).unwrap_or_default();
        if got != want.trim_end() {
            failed.push(name.clone());
        }
    }
    assert!(failed.is_empty(), "fixtures differ: {failed:?}");
}

fn num(v: &Value) -> rust_decimal::Decimal {
    v.to_string().parse().unwrap()
}

/// Every breakdown adds up to its net profit, and the net profit is the lower outcome.
#[test]
fn breakdowns_add_up() {
    for e in fs::read_dir(root().join("cases")).unwrap() {
        let r: Value =
            serde_json::from_str(&run(&fs::read_to_string(e.unwrap().path()).unwrap())).unwrap();
        if r.get("error").is_some() {
            continue;
        }
        let lines = r["breakdown"].as_array().unwrap();
        let (net, parts) = lines.split_last().unwrap();
        let sum: rust_decimal::Decimal = parts.iter().map(|l| num(&l["amount"])).sum();
        assert_eq!(sum, num(&net["amount"]), "{}", r["event"]);
        let o = &r["by_outcome"];
        let low =
            num(&o["event_happens"]["profit"]).min(num(&o["event_does_not_happen"]["profit"]));
        assert_eq!(low, num(&r["guaranteed_profit"]), "{}", r["event"]);
    }
}
