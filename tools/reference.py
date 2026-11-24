"""Independent check of the fixture suite.

Re-prices every fixture case pair by pair in exact fractions, with the venue rules
written out by hand from venue-specs' notes (only rates and steps are read from the
venue-specs bundle), and compares every curve point, the optimum, the break-even size,
the breakdown and the returns with arbcalc's output in fixtures/expected.

Usage: python tools/reference.py path/to/venue-specs.json
"""

from __future__ import annotations

import json
import math
import sys
from fractions import Fraction as F
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]


def spec(bundle, venue, product):
    return next(s for s in bundle["specs"] if s["venue"] == venue and s["product"] == product)


def ceil_to(x, step):
    return math.ceil(x / step) * step


def floor_to(x, step):
    return math.floor(x / step) * step


def round_half_away(x, step):
    q = x / step
    n = math.floor(abs(q) + F(1, 2))
    return (n if q >= 0 else -n) * step


class Leg:
    def __init__(self, leg, bundle):
        self.venue = leg["venue"]
        self.yes = leg["side"] == "yes"
        role = leg.get("role", "taker")
        if "levels" in leg:
            levels = [(F(str(p)), F(str(s))) for p, s in leg["levels"]]
        else:
            levels = [(F(str(leg["price"])), F(str(leg["size"])))]
        self.book = sorted(levels, key=lambda x: x[0])
        if self.venue == "kalshi":
            s = spec(bundle, "kalshi", "binary")["fees"]["rounding"]
            self.trade_step = F(str(s["trade_fee_step"]["value"]))
            self.balance = F(str(s["balance_step"]["value"]))
            # The leg's own model fee formula: a product of a coefficient and C*P*(1-P),
            # or a constant, as in the fixtures.
            f = leg["model_fee"].replace(" ", "")
            if f.endswith("*C*P*(1-P)"):
                self.model = lambda c, p, k=F(f.split("*")[0]): k * c * p * (1 - p)
            else:
                self.model = lambda c, p, k=F(f): k
        elif self.venue == "polymarket":
            s = spec(bundle, "polymarket", "binary")["fees"]
            self.rate = F(str(s["categories"][leg["category"]]["rate"]["value"])) if role == "taker" else F(0)
            self.step = F(str(s["rounding"]["step"]["value"]))

    def fill(self, c, p):
        """(model, trade, charged) for one fill."""
        if self.venue == "kalshi":
            m = self.model(c, p)
            t = ceil_to(m, self.trade_step)
            charged = ceil_to(c * p + t, self.balance) - c * p
            return m, t, charged
        if self.venue == "polymarket":
            f = round_half_away(c * self.rate * p * (1 - p), self.step)
            return f, f, f
        return F(0), F(0), F(0)  # HIP-4: fees only at settlement, and zero there today

    def totals(self, n):
        cost = model = fees = carry = F(0)
        left = n
        last = None
        for i, (p, size) in enumerate(self.book):
            take = min(left, size) if i + 1 < len(self.book) else left
            if take <= 0:
                break
            m, t, ch = self.fill(take, p)
            net = ch
            if self.venue == "kalshi":
                carry += ch - t
                rebate = max(min(floor_to(carry, self.balance), ch), 0)
                carry -= rebate
                net = ch - rebate
            cost += p * take
            model += m
            fees += net
            last = p
            left -= take
        return cost, model, fees, last


def days(d):
    from datetime import date

    return date.fromisoformat(d).toordinal()


def check(name, case, out, bundle):
    legs = [Leg(l, bundle) for l in case["legs"]]
    fixed = case.get("fixed_costs", 0)
    fixed = sum(F(str(x)) for x in fixed) if isinstance(fixed, list) else F(str(fixed))
    depth = min(sum(s for _, s in l.book) for l in legs)
    nmax = math.floor(depth)

    def g(n):
        t = [l.totals(F(n)) for l in legs]
        return n - sum(x[0] + x[2] for x in t) - fixed, t

    curve = [g(n)[0] for n in range(1, nmax + 1)]
    best = max(curve)
    best_n = curve.index(best) + 1
    be = next((i + 1 for i, v in enumerate(curve) if v >= 0), None)
    errs = []
    want = lambda k, a, b: a == b or errs.append(f"{k}: reference {a} vs arbcalc {b}")
    for n, v in out["curve"]:
        want(f"curve[{n}]", curve[n - 1], F(str(v)))
    profitable = best > 0
    want("optimal_size", best_n if profitable else None, out["sizing"]["optimal_size"])
    want("break_even_size", be, out["sizing"]["break_even_size"])
    n = best_n if profitable else nmax
    gn, t = g(n)
    want("guaranteed_profit", gn, F(str(out["guaranteed_profit"])))
    for k in range(2):
        want(f"leg {k} entry fees", t[k][2], F(str(out["legs"][k]["entry_fees"])))
        want(f"leg {k} model fee", t[k][1], F(str(out["legs"][k]["model_fee"])))
    capital = sum(x[0] + x[2] for x in t) + fixed
    if "payout_days" in case:
        span = case["payout_days"]
    else:
        span = max(days(l["payout_date"]) for l in case["legs"]) - days(case["rules_date"])
    roc = gn / capital
    want("roc", round(roc, 12), round(F(str(out["returns"]["roc"])), 12))
    want("apr", round(roc * 365 / span, 12), round(F(str(out["returns"]["apr"])), 12))
    return errs, len(out["curve"])


def main():
    bundle = json.loads(Path(sys.argv[1]).read_text())
    failed = 0
    for path in sorted((ROOT / "fixtures/cases").glob("*.json")):
        out = json.loads((ROOT / "fixtures/expected" / path.name).read_text())
        if "error" in out:
            continue
        errs, points = check(path.name, json.loads(path.read_text()), out, bundle)
        failed += bool(errs)
        print(f"{'FAIL' if errs else 'ok  '} {path.name} ({points} curve points)")
        for e in errs[:5]:
            print("     ", e)
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
