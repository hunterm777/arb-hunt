"""The shared fixture suite must give exactly the native text, byte for byte."""

import json
from pathlib import Path

import pytest

import arbcalc

FIXTURES = Path(__file__).resolve().parents[3] / "fixtures"
NAMES = sorted(p.name for p in (FIXTURES / "cases").glob("*.json"))


def run(text):
    try:
        return arbcalc.check_json(text)
    except ValueError as e:
        # Same shape and escaping as the native suite (serde_json): {"error":"..."}.
        return json.dumps({"error": str(e)}, separators=(",", ":"), ensure_ascii=False)


def test_suite_is_present():
    assert len(NAMES) >= 12


@pytest.mark.parametrize("name", NAMES)
def test_case_matches_native(name):
    got = run((FIXTURES / "cases" / name).read_text())
    assert got == (FIXTURES / "expected" / name).read_text().rstrip()
