# arbcalc

**Most arbs die after fees. Check yours.**

arbcalc answers one question: given the prices for the same event on Kalshi, Polymarket and Hyperliquid HIP-4, does a cross-venue arb still make money after each venue's fees and rounding, and what does it return per year while the capital waits for the payout?

You type in the prices and the size on offer. There is no API and no market data. Fee and rounding rules come from [venue-specs](https://github.com/hunterm777/venue-specs), so arbcalc follows a venue's fee change when venue-specs does, and every result says which venue-specs version and rules date produced it.

**Page:** [hunterm777.github.io/arb-hunt](https://hunterm777.github.io/arb-hunt/) (dark mode, runs entirely in your browser, nothing is fetched after it loads)

## What it checks

An arb here is YES on one venue and NO on the other for the same binary event. If both markets resolve the same way, exactly one leg pays out whatever happens, so

```
profit = payout - cost of both legs - entry fees - settlement fees - fixed costs
```

For each check arbcalc gives:

- **Verdict:** "Profitable: $X guaranteed on N pairs" or "Not profitable after fees: loses $X", in large type.
- **Cost breakdown:** gross edge, each venue's fee, Kalshi's fee rounding as its own line, settlement fees, fixed costs, ending at net profit.
- **By outcome:** net profit if the event happens and if it doesn't. HIP-4 charges its fee at settlement, so the two can differ; the guaranteed profit is the lower.
- **Sizing:** with depth entered as price levels, both books are walked together one pair at a time. You get the profit-maximising size, the average price per leg, the last profitable pair and the break-even size where fixed costs are covered.
- **Returns:** capital required (cost of both legs, entry fees and fixed costs), return on capital, days to payout (the later leg's payout date) and simple APR.
- **Profit vs size chart:** guaranteed profit as the number of pairs grows, with the optimum and break-even marked.

The page states, above the verdict, that the math assumes both legs resolve the same way, and lists what can break that: resolution mismatch, legging, payout timing, access and fee changes. arbcalc does not score or price those risks.

## Fees and rounding

Every rule is read from venue-specs at the rules date you pick (today by default). None is written into arbcalc.

| Venue | When charged | Rule (venue-specs) |
| --- | --- | --- |
| Kalshi | Each fill | Model fee rounded up to the trade-fee step, then the charge rounded up to the cent, with the per-order rounding accumulator. |
| Polymarket | Each fill, takers only | `C × rate × P × (1 − P)` with the rate for the market's category, rounded to 5 decimals. |
| Hyperliquid HIP-4 | Close or settlement only | Opening fills pay nothing; the settlement fee is evaluated under the rules in force on the payout date. |

Where venue-specs marks a value unverified, arbcalc says so and does not guess:

- **Kalshi's model fee formula** is unverified in venue-specs, so you enter it yourself in `C` (contracts) and `P` (price). The page prefills the taker formula `0.07 * C * P * (1 - P)` and shows venue-specs' reason next to it. Once venue-specs carries a sourced formula, it is used and your entry is ignored, with a note.
- **Payout per contract** on Kalshi and Polymarket is unverified in venue-specs, so the winning side pays $1 per contract, as the arb math assumes, and the result notes it. HIP-4 payout comes from venue-specs.
- **Minimum order sizes** are unverified for all three venues, so none is applied. Price steps and size steps that venue-specs does carry are enforced.

Each fill is one price level, as entered. Splitting an order into more fills can cost more on Kalshi because each fill's fee is rounded up, and the rounding line shows that cost.

## Python

The same Rust core is available from Python, so a check returns exactly the page's numbers. Money figures come back as `decimal.Decimal`.

```python
import arbcalc as ac

result = ac.check(
    leg_a=ac.Leg("kalshi", side="yes", levels=[(0.41, 300), (0.42, 500)],
                 model_fee="0.07 * C * P * (1 - P)"),
    leg_b=ac.Leg("polymarket", side="no", category="economics",
                 levels=[(0.55, 200), (0.56, 800)]),
    fixed_costs=2.00,
    payout_days=45,
    rules_date="2026-11-24",
)
result.verdict        # 'Profitable: $0.213 guaranteed on 300 pairs'
result.optimal_size   # 300
result.break_even_size  # 188
result.apr            # Decimal('0.005762980605')
result.breakdown      # [('Gross edge', Decimal('11')), ('Kalshi fee', Decimal('-5.0799')), ...]
```

A leg takes `levels` or one `price` and `size`, plus `category`, `role` (`"taker"`, the default, or `"maker"`), `model_fee` and `payout_date`. `check` takes `payout_days` or a payout date on each leg, `fixed_costs` as one amount or a list, and `rules_date` (today, UTC, by default). Bad input raises `ValueError` with one sentence on what to fix. `ac.meta()` lists the venue-specs version, each venue's fee categories and which fees venue-specs marks unverified.

Prices in these examples are illustrative inputs, not market data.

The package needs Python 3.10 or later and is not on PyPI. Build it from source with [maturin](https://www.maturin.rs/):

```bash
pip install maturin
maturin build --release -m crates/python/Cargo.toml --out wheels
pip install wheels/*.whl
```

## Limits

- Two legs, one on each of two venues, for one binary event. Same-venue arbs, multi-outcome arbs and three-venue legs are not in v1.
- One check walks at most 200,000 pairs (about 1.5 s in a browser), since every pair is priced exactly.
- APR is simple: return on capital × 365 / days to payout.

## How it's built

```
crates/core     all the math, in Rust, with exact decimals and no I/O; depends on venue-specs
crates/wasm     wasm-bindgen wrapper for the page (no logic of its own)
crates/python   PyO3 wrapper built with maturin (no logic of its own)
web/            TypeScript (strict) + Vite page, ThesisGrid's dark design tokens, WASM inlined
fixtures/       shared input cases and the exact expected reports
tools/          an independent re-pricing of the fixtures in exact fractions
```

venue-specs is pinned to a commit in `Cargo.toml`, so a rules change reaches arbcalc as a reviewed bump. These checks run in CI on every push:

- **Parity:** every fixture case runs through native Rust, the WASM build (in Node) and the Python wheel, and each must give the expected report byte for byte, errors included.
- **Independent check:** `tools/reference.py` re-prices every fixture pair by pair in exact fractions and compares every curve point, the optimum, the break-even size, the fees and the returns.
- **Page:** a headless-Chrome check types the PRD's example into the page and checks the verdict, times the largest check, and confirms there are no requests after load, no CSP violations and no console errors.
- **No network:** the Python tests fail if anything opens a socket. The page's build check rejects any fetch, XHR, WebSocket or remote asset, and its CSP sets `connect-src 'none'`.
- **Quality:** `cargo fmt`, `cargo clippy -D warnings`, TypeScript strict mode.

The release workflow runs only when a version tag is pushed. It builds wheels and a source distribution and attaches them to a GitHub release; it publishes nothing to PyPI or crates.io.

### Run it locally

```bash
# Rust core and fixtures
cargo test --workspace --exclude arbcalc-python

# The page (needs wasm-bindgen-cli 0.2.130 and the wasm32-unknown-unknown target)
cd web && npm ci && npm run wasm && npm run dev

# The Python package
pip install maturin && maturin develop --release -m crates/python/Cargo.toml
pytest crates/python/tests
```

## License

[MIT](LICENSE)
