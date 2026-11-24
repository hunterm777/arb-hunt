"""The Python API: the PRD's own example and its acceptance cases, through Leg/check."""

import datetime as dt
from decimal import Decimal as D

import pytest

import arbcalc as ac

KALSHI_FEE = "0.07 * C * P * (1 - P)"  # venue-specs marks Kalshi's model fee unverified
RULES = "2026-11-24"


def test_prd_python_example():
    r = ac.check(
        leg_a=ac.Leg("kalshi", side="yes", levels=[(0.41, 300), (0.42, 500)], model_fee=KALSHI_FEE),
        leg_b=ac.Leg("polymarket", side="no", category="economics", levels=[(0.55, 200), (0.56, 800)]),
        fixed_costs=2.00,
        payout_days=45,
        rules_date=RULES,
    )
    assert r.verdict == "Profitable: $0.213 guaranteed on 300 pairs"
    assert r.optimal_size == 300
    assert r.break_even_size == 188
    assert r.apr == D("0.005762980605")
    assert isinstance(r.guaranteed_profit, D)
    assert r.breakdown[-1] == ("Net profit", D("0.213"))
    assert sum(a for _, a in r.breakdown[:-1]) == r.guaranteed_profit
    assert r.provenance["venue_specs"] == ac.meta()["venue_specs"]


def test_prd_case_3_two_cent_edge_loses():
    r = ac.check(
        ac.Leg("kalshi", side="yes", price=0.48, size=100, model_fee=KALSHI_FEE),
        ac.Leg("polymarket", side="no", price=0.50, size=100, category="sports"),
        payout_days=30,
        rules_date=RULES,
    )
    assert not r.profitable
    assert r.guaranteed_profit == D("-1")
    assert r.verdict == "Not profitable after fees: loses $1.00"


def test_decimals_and_text_cross_exactly():
    a = ac.check(
        ac.Leg("kalshi", "yes", price=D("0.41"), size="100", model_fee=KALSHI_FEE),
        ac.Leg("polymarket", "no", price="0.55", size=D(100), category="economics"),
        fixed_costs=[D("1.25"), 0.75],
        payout_days=45,
        rules_date=dt.date(2026, 11, 24),
    )
    b = ac.check(
        ac.Leg("kalshi", "yes", price=0.41, size=100, model_fee=KALSHI_FEE),
        ac.Leg("polymarket", "no", price=0.55, size=100, category="economics"),
        fixed_costs=2,
        payout_days=45,
        rules_date=RULES,
    )
    assert a.json == b.json


def test_rules_date_defaults_to_today_utc():
    r = ac.check(
        ac.Leg("kalshi", "yes", price=0.41, size=10, model_fee=KALSHI_FEE),
        ac.Leg("hyperliquid", "no", price=0.55, size=10),
        payout_days=10,
    )
    assert r.provenance["rules_date"] == dt.datetime.now(dt.timezone.utc).date().isoformat()


def test_errors_say_what_to_fix():
    with pytest.raises(ValueError, match="model fee"):
        ac.check(
            ac.Leg("kalshi", "yes", price=0.41, size=100),
            ac.Leg("polymarket", "no", price=0.55, size=100, category="economics"),
            payout_days=45,
            rules_date=RULES,
        )
    with pytest.raises(TypeError):
        ac.Leg("kalshi", "yes", price=True, size=1).to_dict()


def test_meta_lists_categories_and_unverified_fees():
    m = ac.meta()
    assert "economics" in m["venues"]["polymarket"]["categories"]
    assert m["venues"]["kalshi"]["model_fee_unverified"]["taker"]
    assert ac.__version__ == "0.1.0"
