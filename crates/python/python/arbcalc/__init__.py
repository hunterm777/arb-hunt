"""Most arbs die after fees. Check yours.

``arbcalc`` checks a cross-venue binary arb (YES on one venue, NO on another, same event)
on Kalshi, Polymarket and Hyperliquid HIP-4: net profit after each venue's exact fees and
rounding, the profit-maximising size and break-even size from the depth you enter, and
the return on capital and APR to the payout date. Fee and rounding rules come from
venue-specs, compiled in. The math is the Rust core the page runs, so both give the same
numbers. No network access.

Every money figure in a result is a :class:`decimal.Decimal`, exactly as the core
computed it.
"""

from __future__ import annotations

import datetime as _dt
import json as _json
from dataclasses import dataclass
from decimal import Decimal
from typing import Any, Iterable, Sequence

from . import _arbcalc

__version__: str = _arbcalc.__version__

__all__ = ["Leg", "Result", "check", "check_json", "meta", "__version__"]

Number = int | float | Decimal | str


def _num(x: Number) -> Any:
    # Decimals and text cross into the core as their exact digits; floats by their
    # shortest repr (0.41 stays 0.41).
    if isinstance(x, bool):
        raise TypeError("a number is expected, not a bool")
    if isinstance(x, (Decimal, str)):
        return str(x)
    if isinstance(x, (int, float)):
        return x
    raise TypeError(f"a number is expected, not {type(x).__name__}")


@dataclass(frozen=True)
class Leg:
    """One side of the arb.

    Give ``levels`` as ``(price, size)`` pairs, or one ``price`` and ``size``.
    ``category`` is the venue's fee category (Polymarket). ``model_fee`` is your formula
    in ``C`` (contracts) and ``P`` (price) for a fee venue-specs has not verified yet
    (Kalshi), such as ``"0.07 * C * P * (1 - P)"``. ``payout_date`` is ``YYYY-MM-DD``.
    """

    venue: str
    side: str = "yes"
    levels: Sequence[tuple[Number, Number]] | None = None
    price: Number | None = None
    size: Number | None = None
    category: str | None = None
    role: str = "taker"
    model_fee: str | None = None
    payout_date: str | None = None

    def to_dict(self) -> dict[str, Any]:
        d: dict[str, Any] = {"venue": self.venue, "side": self.side, "role": self.role}
        if self.levels is not None:
            d["levels"] = [[_num(p), _num(s)] for p, s in self.levels]
        if self.price is not None:
            d["price"] = _num(self.price)
        if self.size is not None:
            d["size"] = _num(self.size)
        for k in ("category", "model_fee", "payout_date"):
            if getattr(self, k) is not None:
                d[k] = getattr(self, k)
        return d


class Result:
    """A check's report. ``data`` holds the whole report; the attributes are shortcuts."""

    def __init__(self, text: str):
        self.json: str = text
        self.data: dict[str, Any] = _json.loads(text, parse_float=Decimal, parse_int=Decimal)
        d = self.data
        self.event: str = d["event"]
        self.verdict: str = d["verdict"]
        self.profitable: bool = d["profitable"]
        self.pairs: int = int(d["pairs"])
        self.guaranteed_profit: Decimal = d["guaranteed_profit"]
        self.breakdown: list[tuple[str, Decimal]] = [(b["label"], b["amount"]) for b in d["breakdown"]]
        self.by_outcome: dict[str, Any] = d["by_outcome"]
        self.legs: list[dict[str, Any]] = d["legs"]
        s = d["sizing"]
        self.max_pairs: int = int(s["max_pairs"])
        self.optimal_size: int | None = None if s["optimal_size"] is None else int(s["optimal_size"])
        self.break_even_size: int | None = None if s["break_even_size"] is None else int(s["break_even_size"])
        self.average_price: tuple[Decimal, Decimal] = tuple(s["average_price"])  # type: ignore[assignment]
        self.last_profitable_pair: dict[str, Any] | None = s["last_profitable_pair"]
        r = d["returns"]
        self.capital: Decimal = r["capital"]
        self.roc: Decimal = r["roc"]
        self.days_to_payout: int = int(r["days_to_payout"])
        self.apr: Decimal = r["apr"]
        self.curve: list[tuple[int, Decimal]] = [(int(n), g) for n, g in d["curve"]]
        self.provenance: dict[str, Any] = d["provenance"]
        self.notes: list[str] = d["notes"]

    def __repr__(self) -> str:
        return f"<arbcalc.Result {self.verdict!r}>"


def check(
    leg_a: Leg,
    leg_b: Leg,
    fixed_costs: Number | Iterable[Number] = 0,
    payout_days: int | None = None,
    rules_date: str | _dt.date | None = None,
    event: str = "",
) -> Result:
    """Check one arb. ``payout_days`` overrides the legs' payout dates. ``rules_date``
    picks which date's venue rules apply (today, UTC, by default). Raises ``ValueError``
    with a sentence that says what to fix."""
    if rules_date is None:
        rules_date = _dt.datetime.now(_dt.timezone.utc).date()
    if isinstance(fixed_costs, (int, float, Decimal, str)):
        costs: Any = _num(fixed_costs)
    else:
        costs = [_num(c) for c in fixed_costs]
    doc: dict[str, Any] = {
        "event": event,
        "legs": [leg_a.to_dict(), leg_b.to_dict()],
        "fixed_costs": costs,
        "rules_date": str(rules_date),
    }
    if payout_days is not None:
        doc["payout_days"] = payout_days
    return Result(_arbcalc.check_json(_json.dumps(doc)))


def check_json(text: str) -> str:
    """The core's JSON interface, as the page and the fixture suite use it."""
    return _arbcalc.check_json(text)


def meta() -> dict[str, Any]:
    """The venue-specs version compiled in, fee categories per venue, and which model
    fees venue-specs marks unverified (with the reason)."""
    return _json.loads(_arbcalc.meta())
