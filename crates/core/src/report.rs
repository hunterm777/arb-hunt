//! The check: pair both books, find the optimum and break-even sizes, and build the
//! report the page and the Python package show.

use serde_json::{json, Value};
use venue_specs::Decimal;

use crate::dates::{date, days};
use crate::error::{Error, Result};
use crate::fees::{LegRules, Rules};
use crate::leg::Leg;
use crate::num::{dollars, to_json};
use crate::outcomes::{outcome, payouts, Outcome};
use crate::pairing::{LegTotals, Walker};
use crate::returns::{returns, Returns};

/// The most pairs one check walks. Each pair is priced exactly, with the fee of its
/// partial fill evaluated by venue-specs, so the walk is linear in the size; at this cap
/// a check takes about 1.5 s in a browser.
pub const MAX_PAIRS: u64 = 200_000;

/// Most points kept for the profit vs size chart.
pub const CURVE_POINTS: u64 = 600;

/// Everything a check takes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Input {
    /// Free text for the user's own reference.
    pub event: String,
    /// Leg A and leg B.
    pub legs: [Leg; 2],
    /// Deposits, withdrawals and transfers needed to fund both legs, in dollars.
    pub fixed_costs: Decimal,
    /// Days to payout for both legs; use instead of per-leg payout dates.
    pub payout_days: Option<i64>,
    /// Which day's venue rules to apply, `YYYY-MM-DD`. Days to payout count from it.
    pub rules_date: String,
}

/// A line of the cost breakdown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// What the line is.
    pub label: String,
    /// Signed dollars: costs are negative.
    pub amount: Decimal,
}

/// The pair that sets the optimum.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LastPair {
    /// Its number, 1-based.
    pub pair: u64,
    /// Price paid on each leg for it.
    pub prices: [Decimal; 2],
    /// What it adds to profit, before fixed costs.
    pub marginal_profit: Decimal,
}

/// The result of a check.
#[derive(Debug, Clone, PartialEq)]
pub struct Report {
    /// The input's event label.
    pub event: String,
    /// "Profitable: ..." or "Not profitable after fees: ...".
    pub verdict: String,
    /// True when some size makes a guaranteed profit above zero.
    pub profitable: bool,
    /// Pairs the verdict, breakdown and returns are for: the optimum when profitable,
    /// otherwise every pair available.
    pub pairs: u64,
    /// The lower of the two outcomes' profits.
    pub guaranteed_profit: Decimal,
    /// Gross edge, each fee, rounding cost and fixed cost, ending at net profit.
    pub breakdown: Vec<Line>,
    /// `[event happens, event does not happen]`.
    pub outcomes: [Outcome; 2],
    /// Each leg's totals at `pairs`.
    pub legs: [LegTotals; 2],
    /// Payout date per leg.
    pub payout_dates: [String; 2],
    /// Pairs available (the shallower book, whole contracts).
    pub max_pairs: u64,
    /// The size with the highest guaranteed profit, when that profit is above zero.
    pub optimal_size: Option<u64>,
    /// The smallest size whose guaranteed profit covers the fixed costs.
    pub break_even_size: Option<u64>,
    /// The last pair at the optimum.
    pub last_profitable_pair: Option<LastPair>,
    /// Capital, ROC, days to payout and APR at `pairs`.
    pub returns: Returns,
    /// `(pairs, guaranteed profit)`, at most [`CURVE_POINTS`] points plus the marks.
    pub curve: Vec<(u64, Decimal)>,
    /// venue-specs data version.
    pub venue_specs_version: String,
    /// Rules date applied.
    pub rules_date: String,
    /// Where each leg's rules came from.
    pub provenance: [Value; 2],
    /// Which rules were filled in or skipped, and why.
    pub notes: Vec<String>,
}

fn check_legs(input: &Input) -> Result<()> {
    let [a, b] = &input.legs;
    if a.venue == b.venue {
        return Err(Error::input(
            "both legs are on one venue: same-venue arbs are not part of v1",
        ));
    }
    if a.side == b.side {
        return Err(Error::input(
            "both legs buy the same side: buy YES on one venue and NO on the other",
        ));
    }
    if input.fixed_costs < Decimal::ZERO {
        return Err(Error::input("fixed costs cannot be negative"));
    }
    a.check_levels("Leg A")?;
    b.check_levels("Leg B")
}

fn payout_days(input: &Input) -> Result<([i64; 2], i64)> {
    let start = days(&input.rules_date)?;
    let mut out = [0; 2];
    for (k, leg) in input.legs.iter().enumerate() {
        out[k] = match (input.payout_days, &leg.payout_date) {
            (Some(_), Some(_)) => {
                return Err(Error::input(
                    "give payout days or a payout date per leg, not both",
                ))
            }
            (Some(d), None) => start + d,
            (None, Some(p)) => days(p)?,
            (None, None) => {
                return Err(Error::input(format!(
                    "Leg {}: enter its payout date",
                    ["A", "B"][k]
                )))
            }
        };
    }
    let span = out[0].max(out[1]) - start;
    if span <= 0 {
        return Err(Error::input(
            "the later payout date must be after the rules date",
        ));
    }
    Ok((out, span))
}

struct Pass<'a> {
    rules: &'a [LegRules; 2],
    pay: [[Decimal; 2]; 2],
    fixed: Decimal,
}

impl Pass<'_> {
    fn at(&self, w: &mut [Walker<'_>; 2], n: u64) -> Result<([LegTotals; 2], [Outcome; 2])> {
        let n = Decimal::from(n);
        let t = [w[0].totals(n)?, w[1].totals(n)?];
        let o = [
            outcome(0, &t, self.rules, &self.pay, self.fixed)?,
            outcome(1, &t, self.rules, &self.pay, self.fixed)?,
        ];
        Ok((t, o))
    }
}

fn guaranteed(o: &[Outcome; 2]) -> Decimal {
    o[0].profit.min(o[1].profit)
}

fn walkers<'a>(input: &Input, rules: &'a [LegRules; 2]) -> [Walker<'a>; 2] {
    [
        Walker::new(&rules[0], input.legs[0].book()),
        Walker::new(&rules[1], input.legs[1].book()),
    ]
}

fn curve_keep(n: u64, max: u64) -> bool {
    max <= CURVE_POINTS || n == 1 || n == max || (n * CURVE_POINTS) % max < CURVE_POINTS
}

/// Run a check against `rules`.
pub fn check_with(rules: &dyn Rules, input: &Input) -> Result<Report> {
    check_legs(input)?;
    let (pdays, span) = payout_days(input)?;
    let payout_dates = [date(pdays[0]), date(pdays[1])];
    let leg_rules = [
        LegRules::new(rules, &input.legs[0], &input.rules_date, &payout_dates[0])?,
        LegRules::new(rules, &input.legs[1], &input.rules_date, &payout_dates[1])?,
    ];
    for (k, leg) in input.legs.iter().enumerate() {
        for l in &leg.levels {
            leg_rules[k].check_level(["Leg A", "Leg B"][k], l.price, l.size)?;
        }
    }
    let depth = input.legs[0].depth().min(input.legs[1].depth()).floor();
    let max_pairs = u64::try_from(depth.mantissa() / 10i128.pow(depth.scale())).unwrap_or(u64::MAX);
    if max_pairs < 1 {
        return Err(Error::input(
            "both legs need at least one whole contract to pair",
        ));
    }
    if max_pairs > MAX_PAIRS {
        return Err(Error::input(format!(
            "{max_pairs} pairs is over the {MAX_PAIRS} a check walks: enter less depth"
        )));
    }
    let pass = Pass {
        rules: &leg_rules,
        pay: payouts(&input.legs, &leg_rules)?,
        fixed: input.fixed_costs,
    };

    // Price every size from 1 to max_pairs exactly; rounding per fill means profit is
    // not smooth in N, so the optimum is the best of all of them.
    let mut w = walkers(input, &leg_rules);
    let mut best: Option<(u64, Decimal)> = None;
    let mut break_even = None;
    let mut curve = Vec::new();
    let mut prev = -input.fixed_costs;
    let mut marginal_at_best = Decimal::ZERO;
    for n in 1..=max_pairs {
        let (_, o) = pass.at(&mut w, n)?;
        let g = guaranteed(&o);
        if best.is_none_or(|(_, b)| g > b) {
            best = Some((n, g));
            marginal_at_best = g - prev;
        }
        if break_even.is_none() && g >= Decimal::ZERO {
            break_even = Some(n);
        }
        if curve_keep(n, max_pairs) {
            curve.push((n, g));
        }
        prev = g;
    }
    let (best_n, best_g) = best.expect("max_pairs >= 1");
    let profitable = best_g > Decimal::ZERO;
    let pairs = if profitable { best_n } else { max_pairs };
    for mark in [Some(best_n), break_even].into_iter().flatten() {
        if let Err(i) = curve.binary_search_by_key(&mark, |p| p.0) {
            let (_, o) = pass.at(&mut walkers(input, &leg_rules), mark)?;
            curve.insert(i, (mark, guaranteed(&o)));
        }
    }

    let (totals, outcomes) = pass.at(&mut walkers(input, &leg_rules), pairs)?;
    let g = guaranteed(&outcomes);
    let verdict = if profitable {
        format!("Profitable: {} guaranteed on {pairs} pairs", dollars(g))
    } else {
        format!("Not profitable after fees: loses {}", dollars(-g))
    };
    let worst = usize::from(outcomes[1].profit < outcomes[0].profit);
    let breakdown = breakdown(input, &leg_rules, &totals, &outcomes[worst], g);
    let mut notes: Vec<String> = leg_rules.iter().flat_map(|r| r.notes().to_vec()).collect();
    for (leg, r) in input.legs.iter().zip(&leg_rules) {
        if !r.has_payout_formula() {
            notes.push(format!(
                "{}: venue-specs has no payout rule yet, so the winning side pays $1 per contract, as the arb math assumes.",
                leg.venue.label()
            ));
        }
    }
    Ok(Report {
        event: input.event.clone(),
        verdict,
        profitable,
        pairs,
        guaranteed_profit: g,
        breakdown,
        outcomes,
        legs: totals,
        provenance: [
            leg_rules[0].provenance(&payout_dates[0]),
            leg_rules[1].provenance(&payout_dates[1]),
        ],
        payout_dates,
        max_pairs,
        optimal_size: profitable.then_some(best_n),
        break_even_size: break_even,
        last_profitable_pair: profitable.then_some(LastPair {
            pair: best_n,
            prices: [totals[0].last_price, totals[1].last_price],
            marginal_profit: marginal_at_best,
        }),
        returns: returns(g, &totals, input.fixed_costs, span),
        curve,
        venue_specs_version: rules.version(),
        rules_date: input.rules_date.clone(),
        notes,
    })
}

fn breakdown(
    input: &Input,
    rules: &[LegRules; 2],
    t: &[LegTotals; 2],
    worst: &Outcome,
    net: Decimal,
) -> Vec<Line> {
    let line = |label: String, amount: Decimal| Line { label, amount };
    let name = |k: usize| input.legs[k].venue.label();
    let mut out = vec![line(
        "Gross edge".into(),
        worst.payout - t[0].cost - t[1].cost,
    )];
    for (k, tk) in t.iter().enumerate() {
        out.push(line(format!("{} fee", name(k)), -tk.model));
    }
    for (k, (r, tk)) in rules.iter().zip(t).enumerate() {
        if r.has_model() {
            out.push(line(format!("{} fee rounding", name(k)), -tk.rounding()));
        }
    }
    for (k, r) in rules.iter().enumerate() {
        if r.charges_at_settlement() {
            out.push(line(
                format!("{} settlement fee", name(k)),
                -worst.settlement_fees[k],
            ));
        }
    }
    out.push(line("Fixed costs".into(), -input.fixed_costs));
    out.push(line("Net profit".into(), net));
    out
}

impl Report {
    /// The report as JSON, every amount an exact decimal.
    pub fn to_json(&self) -> Value {
        let d = to_json;
        let outcome = |o: &Outcome| {
            json!({
                "payout": d(o.payout),
                "settlement_fees": [d(o.settlement_fees[0]), d(o.settlement_fees[1])],
                "profit": d(o.profit),
            })
        };
        let leg = |k: usize| {
            let t = &self.legs[k];
            json!({
                "contracts": d(t.contracts),
                "cost": d(t.cost),
                "average_price": d(t.average_price()),
                "model_fee": d(t.model),
                "rounding": d(t.rounding()),
                "entry_fees": d(t.fees),
                "fills": t.fills,
                "last_price": d(t.last_price),
                "payout_date": self.payout_dates[k],
            })
        };
        json!({
            "event": self.event,
            "verdict": self.verdict,
            "profitable": self.profitable,
            "pairs": self.pairs,
            "guaranteed_profit": d(self.guaranteed_profit),
            "breakdown": self.breakdown.iter().map(|l| json!({"label": l.label, "amount": d(l.amount)})).collect::<Vec<_>>(),
            "by_outcome": {
                "event_happens": outcome(&self.outcomes[0]),
                "event_does_not_happen": outcome(&self.outcomes[1]),
                "guaranteed": d(self.guaranteed_profit),
            },
            "legs": [leg(0), leg(1)],
            "sizing": {
                "max_pairs": self.max_pairs,
                "optimal_size": self.optimal_size,
                "break_even_size": self.break_even_size,
                "average_price": [d(self.legs[0].average_price()), d(self.legs[1].average_price())],
                "last_profitable_pair": self.last_profitable_pair.map(|p| json!({
                    "pair": p.pair,
                    "prices": [d(p.prices[0]), d(p.prices[1])],
                    "marginal_profit": d(p.marginal_profit),
                })),
            },
            "returns": {
                "capital": d(self.returns.capital),
                "roc": d(self.returns.roc),
                "days_to_payout": self.returns.days,
                "apr": d(self.returns.apr),
            },
            "curve": self.curve.iter().map(|(n, g)| json!([n, d(*g)])).collect::<Vec<_>>(),
            "provenance": {
                "venue_specs": self.venue_specs_version,
                "rules_date": self.rules_date,
                "legs": self.provenance,
            },
            "notes": self.notes,
        })
    }
}
