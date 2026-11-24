//! Fees, rounding and payouts for one leg, read from venue-specs. Nothing here holds a
//! rate, a step or a formula: each one is a venue-specs entry, evaluated with its own
//! formula grammar, so a venue change is a venue-specs release, not an arbcalc change.

use serde_json::{json, Value};
use venue_specs::formula::{eval, parse, Expr};
use venue_specs::{Decimal, Inputs, Node, Spec};

use crate::error::{Error, Result};
use crate::leg::{Leg, Venue};
use crate::num::{floor_to, to_json};

/// Where specs come from: the embedded venue-specs bundle, or a test bundle.
pub trait Rules {
    /// The spec for `venue/product` in force at the start of `date`.
    fn spec_at(&self, venue: &str, product: &str, date: &str) -> Result<Spec>;
    /// The venue-specs data version.
    fn version(&self) -> String;
}

/// The venue-specs bundle compiled into arbcalc.
pub struct Embedded;

impl Rules for Embedded {
    fn spec_at(&self, venue: &str, product: &str, date: &str) -> Result<Spec> {
        venue_specs::spec_at(venue, product, date).map_err(|e| Error::rules(e.to_string()))
    }

    fn version(&self) -> String {
        venue_specs::version().to_owned()
    }
}

/// The fee on one fill.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct FillFee {
    /// The fee before any rounding: venue-specs `fees.<role>.model_fee` where the spec has
    /// one, otherwise the fee charged.
    pub model: Decimal,
    /// The fee before rounding to the balance step (`fees.<role>.trade_fee`), or the fee.
    pub trade: Decimal,
    /// The fee charged on the fill (`fees.<role>.fee`), before any per-order rebate.
    pub charged: Decimal,
}

/// Per-order rounding accumulator (venue-specs `fees.rounding.accumulator: per_order`):
/// each fill's rounding overpayment is added, and rebated in whole balance steps, capped
/// so the fill's net fee never drops below `fees.rounding.net_fee_floor`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Accumulator {
    step: Decimal,
    floor: Decimal,
}

impl Accumulator {
    /// Applies one fill to the carried overpayment; returns (net fee, new carry).
    pub fn apply(&self, fill: FillFee, carry: Decimal) -> (Decimal, Decimal) {
        let carry = carry + (fill.charged - fill.trade);
        let room = (fill.charged - self.floor).max(Decimal::ZERO);
        let rebate = floor_to(carry, self.step).min(room).max(Decimal::ZERO);
        (fill.charged - rebate, carry - rebate)
    }
}

/// When a venue charges its fee.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Charged {
    /// On every fill (no `fees.when_charged` entry).
    OnFill,
    /// Only when closing or settling (`closing_or_settling_only`); opening fills are free.
    AtSettlement,
}

/// One leg's rules: the spec in force on the rules date, and the spec in force on the
/// payout date for a fee charged at settlement.
pub struct LegRules {
    venue: Venue,
    role: &'static str,
    entry: Spec,
    settle: Spec,
    category: Option<String>,
    model_override: Option<(String, Expr)>,
    has_model: bool,
    has_trade: bool,
    charged: Charged,
    accumulator: Option<Accumulator>,
    notes: Vec<String>,
}

fn leaf<'a>(spec: &'a Spec, path: &str) -> Option<&'a venue_specs::Leaf> {
    spec.leaf(path)
}

fn text_value(spec: &Spec, path: &str) -> Option<String> {
    leaf(spec, path)?
        .value
        .as_ref()?
        .as_str()
        .map(str::to_owned)
}

fn unverified(spec: &Spec, path: &str) -> Option<String> {
    match spec.get(path) {
        Some(Node::Unverified(u)) => Some(u.reason.clone()),
        _ => None,
    }
}

fn rules_err(venue: Venue, e: venue_specs::Error) -> Error {
    Error::rules(format!("{}: {e}", venue.label()))
}

impl LegRules {
    /// The rules for `leg`, entry fees as of `rules_date` and settlement as of `payout_date`.
    pub fn new(
        rules: &dyn Rules,
        leg: &Leg,
        rules_date: &str,
        payout_date: &str,
    ) -> Result<LegRules> {
        let v = leg.venue;
        let entry = rules.spec_at(v.key(), v.product(), rules_date)?;
        let settle = rules.spec_at(v.key(), v.product(), payout_date)?;
        let role = leg.role.key();
        let model_path = format!("fees.{role}.model_fee");
        let mut notes = Vec::new();

        let charged = match text_value(&entry, "fees.when_charged").as_deref() {
            None => Charged::OnFill,
            Some("closing_or_settling_only") => Charged::AtSettlement,
            Some(other) => {
                return Err(Error::rules(format!(
                    "{}: venue-specs fees.when_charged is {other:?}, which arbcalc does not model",
                    v.label()
                )))
            }
        };

        let has_model = entry.get(&model_path).is_some();
        let model_override = match (&leg.model_fee, unverified(&entry, &model_path)) {
            (Some(f), Some(_)) => {
                let expr = parse(f).map_err(|e| {
                    Error::input(format!("{} model fee formula: {e}", v.label()))
                })?;
                notes.push(format!(
                    "{} {model_path} is unverified in venue-specs; using your formula {f}.",
                    v.label()
                ));
                Some((f.trim().to_owned(), expr))
            }
            (Some(_), None) if has_model => {
                notes.push(format!(
                    "{} {model_path} now has a sourced value in venue-specs; your formula is not used.",
                    v.label()
                ));
                None
            }
            (Some(_), None) => {
                return Err(Error::input(format!(
                    "{}: venue-specs has no {model_path}, so a model fee formula does not apply",
                    v.label()
                )))
            }
            (None, Some(reason)) => {
                return Err(Error::rules(format!(
                    "{} {model_path} is unverified in venue-specs ({reason}) Enter the model fee formula for this leg, in C and P.",
                    v.label()
                )))
            }
            (None, None) => None,
        };

        let category = match (&leg.category, entry.get("fees.categories")) {
            (Some(c), Some(Node::Map(rows))) => {
                if !rows.contains_key(c.as_str()) {
                    let keys: Vec<&str> = rows.keys().map(String::as_str).collect();
                    return Err(Error::input(format!(
                        "{}: unknown fee category {c:?}; venue-specs has {}",
                        v.label(),
                        keys.join(", ")
                    )));
                }
                Some(c.clone())
            }
            (None, Some(Node::Map(rows))) => {
                let keys: Vec<&str> = rows.keys().map(String::as_str).collect();
                return Err(Error::input(format!(
                    "{}: choose a fee category ({})",
                    v.label(),
                    keys.join(", ")
                )));
            }
            (Some(c), _) => {
                return Err(Error::input(format!(
                    "{}: venue-specs has no fee categories, so category {c:?} does not apply",
                    v.label()
                )))
            }
            (None, _) => None,
        };

        let accumulator = match text_value(&entry, "fees.rounding.accumulator").as_deref() {
            None => None,
            Some("per_order") => {
                let num = |p: &str| {
                    venue_specs::evaluate(&entry, p, &Inputs::new()).map_err(|e| rules_err(v, e))
                };
                Some(Accumulator {
                    step: num("fees.rounding.balance_step")?,
                    floor: num("fees.rounding.net_fee_floor")?,
                })
            }
            Some(other) => {
                return Err(Error::rules(format!(
                    "{}: venue-specs fees.rounding.accumulator is {other:?}, which arbcalc does not model",
                    v.label()
                )))
            }
        };

        let has_trade = entry.get(&format!("fees.{role}.trade_fee")).is_some();
        Ok(LegRules {
            venue: v,
            role,
            entry,
            settle,
            category,
            model_override,
            has_model,
            has_trade,
            charged,
            accumulator,
            notes,
        })
    }

    /// The per-order accumulator, where the venue has one.
    pub fn accumulator(&self) -> Option<Accumulator> {
        self.accumulator
    }

    /// Notes for the report about which rules were used.
    pub fn notes(&self) -> &[String] {
        &self.notes
    }

    /// True when the venue charges fees only at settlement.
    pub fn charges_at_settlement(&self) -> bool {
        self.charged == Charged::AtSettlement
    }

    /// True when the venue has a model fee separate from the fee charged, so the
    /// difference is the rounding cost.
    pub fn has_model(&self) -> bool {
        self.has_model
    }

    fn inputs(&self, c: Decimal, p: Decimal) -> Inputs {
        let i = Inputs::new()
            .var("C", c)
            .var("P", p)
            .var("Q", c)
            .var("N", c * p);
        match &self.category {
            Some(k) => i.category(k),
            None => i,
        }
    }

    fn eval(&self, spec: &Spec, path: &str, inputs: &Inputs) -> Result<Decimal> {
        venue_specs::evaluate(spec, path, inputs).map_err(|e| rules_err(self.venue, e))
    }

    fn model_from_override(&self, expr: &Expr, f: &str, c: Decimal, p: Decimal) -> Result<Decimal> {
        eval(expr, &mut |name: &str| match name {
            "C" | "contracts" => Ok(c),
            "P" | "price" => Ok(p),
            _ => Err(format!(
                "{name} is not an input: a model fee formula may use C and P"
            )),
        })
        .map_err(|m| {
            Error::input(format!(
                "{} model fee formula {f:?}: {m}",
                self.venue.label()
            ))
        })
    }

    /// The fee on one opening fill of `c` contracts at price `p`.
    pub fn fill(&self, c: Decimal, p: Decimal) -> Result<FillFee> {
        if self.charged == Charged::AtSettlement {
            return Ok(FillFee::default());
        }
        let role = self.role;
        let mut inputs = self.inputs(c, p);
        // Each step is evaluated once and handed to the next as an input, which is how
        // venue-specs resolves a name the caller passes (inputs come first).
        let model = match &self.model_override {
            Some((f, expr)) => Some(self.model_from_override(expr, f, c, p)?),
            None if self.has_model => {
                Some(self.eval(&self.entry, &format!("fees.{role}.model_fee"), &inputs)?)
            }
            None => None,
        };
        if let Some(m) = model {
            inputs = inputs.var("model_fee", m);
        }
        let trade = if self.has_trade {
            let t = self.eval(&self.entry, &format!("fees.{role}.trade_fee"), &inputs)?;
            inputs = inputs.var("trade_fee", t);
            Some(t)
        } else {
            None
        };
        let charged = self.eval(&self.entry, &format!("fees.{role}.fee"), &inputs)?;
        let trade = trade.unwrap_or(charged);
        let model = model.unwrap_or(charged);
        Ok(FillFee {
            model,
            trade,
            charged,
        })
    }

    /// Payout per contract held on `side`, if the event happens or not. HIP-4 reads
    /// `settlement.yes_payout`/`no_payout` with `settle_fraction` 1 or 0. Where venue-specs
    /// has no payout formula (Kalshi and Polymarket mark it unverified), the winning side
    /// pays $1, as the arb math in the PRD states.
    pub fn payout(&self, yes: bool, happens: bool) -> Result<Decimal> {
        let path = if yes {
            "settlement.yes_payout"
        } else {
            "settlement.no_payout"
        };
        if matches!(self.settle.get(path), Some(Node::Leaf(_))) {
            let f = if happens { Decimal::ONE } else { Decimal::ZERO };
            return self.eval(&self.settle, path, &Inputs::new().var("settle_fraction", f));
        }
        Ok(if yes == happens {
            Decimal::ONE
        } else {
            Decimal::ZERO
        })
    }

    /// The fee at settlement on `c` contracts paid `px` each, under the rules in force on
    /// the payout date. Zero for venues that charge on each fill. The settlement fee uses
    /// `fees.taker.fee`, since settlement has no maker side.
    pub fn settlement_fee(&self, c: Decimal, px: Decimal) -> Result<Decimal> {
        if self.charged == Charged::OnFill || c.is_zero() {
            return Ok(Decimal::ZERO);
        }
        let inputs = Inputs::new()
            .var("C", c)
            .var("P", px)
            .var("Q", c)
            .var("N", c * px);
        self.eval(&self.settle, "fees.taker.fee", &inputs)
    }

    /// Check one level against the venue's grid where venue-specs has the rule.
    pub fn check_level(&self, name: &str, price: Decimal, size: Decimal) -> Result<()> {
        let v = self.venue;
        if let Some(d) = leaf(&self.entry, "price_grid.price_decimals").and_then(|l| l.number()) {
            if price.normalize().scale() > d.try_into().unwrap_or(u32::MAX) {
                return Err(Error::input(format!(
                    "{name}: {} prices have at most {d} decimal places (venue-specs price_grid.price_decimals); got {}",
                    v.label(),
                    price.normalize()
                )));
            }
        }
        if let Some(step) = leaf(&self.entry, "size.step").and_then(|l| l.number()) {
            if !(size / step).fract().is_zero() {
                return Err(Error::input(format!(
                    "{name}: {} sizes are in steps of {} contracts (venue-specs size.step); got {}",
                    v.label(),
                    step.normalize(),
                    size.normalize()
                )));
            }
        }
        Ok(())
    }

    /// Where every rule this leg used came from.
    pub fn provenance(&self, payout_date: &str) -> Value {
        let role = self.role;
        let src = |spec: &Spec, path: &str| match spec.leaf(path) {
            Some(l) => json!({
                "path": path,
                "formula": l.formula,
                "source": l.source,
                "as_of": l.as_of,
            }),
            None => json!({ "path": path, "unverified": unverified(spec, path) }),
        };
        let fee_path = format!("fees.{role}.fee");
        let mut out = json!({
            "venue": self.venue.key(),
            "product": self.venue.product(),
            "rules_version": self.entry.version_of_rules,
            "fee": src(&self.entry, &fee_path),
        });
        if self.has_model {
            out["model_fee"] = match &self.model_override {
                Some((f, _)) => {
                    json!({ "path": format!("fees.{role}.model_fee"), "your_formula": f })
                }
                None => src(&self.entry, &format!("fees.{role}.model_fee")),
            };
        }
        if let Some(a) = self.accumulator {
            out["accumulator"] = json!({
                "rule": src(&self.entry, "fees.rounding.accumulator"),
                "balance_step": to_json(a.step),
                "net_fee_floor": to_json(a.floor),
            });
        }
        if let Some(c) = &self.category {
            out["category"] = json!(c);
            out["rate"] = src(&self.entry, &format!("fees.categories.{c}.rate"));
        }
        if self.charged == Charged::AtSettlement {
            out["when_charged"] = src(&self.entry, "fees.when_charged");
            out["settlement_fee"] = json!({
                "rules_date": payout_date,
                "rules_version": self.settle.version_of_rules,
                "rule": src(&self.settle, "fees.taker.fee"),
            });
        }
        out["payout"] = if self.settle.leaf("settlement.yes_payout").is_some() {
            json!({
                "yes": src(&self.settle, "settlement.yes_payout"),
                "no": src(&self.settle, "settlement.no_payout"),
            })
        } else {
            json!({ "rule": "winning side pays $1 per contract (PRD arb math)",
                    "venue_specs": src(&self.settle, "settlement.payout") })
        };
        out
    }
}

impl LegRules {
    /// True when venue-specs has a payout formula for this venue.
    pub fn has_payout_formula(&self) -> bool {
        matches!(
            self.settle.get("settlement.yes_payout"),
            Some(Node::Leaf(_))
        )
    }
}
