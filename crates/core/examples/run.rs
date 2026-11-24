//! Runs a check from a JSON file and prints the report: `cargo run --example run -- case.json`.
fn main() {
    let path = std::env::args().nth(1).expect("usage: run <input.json>");
    let text = std::fs::read_to_string(path).expect("read input");
    match arbcalc_core::api::check_json(&text) {
        Ok(out) => println!("{out}"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
