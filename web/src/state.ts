// The page's inputs, as text exactly as typed, and the last report. Every edit re-runs
// the core once typing pauses.
import * as core from "./core";
import type { Input, InputLeg, Meta, Report, Role, Side, Venue } from "./types";

export interface LegState {
  venue: Venue;
  side: Side;
  role: Role;
  depth: boolean;
  price: string;
  size: string;
  levels: string;
  category: string;
  modelFee: Record<Role, string>;
  payoutDate: string;
}

export interface State {
  event: string;
  legs: [LegState, LegState];
  fixedCosts: string;
  rulesDate: string;
}

export type Result = { ok: Report } | { err: string };

const iso = (d: Date): string =>
  `${d.getFullYear()}-${String(d.getMonth() + 1).padStart(2, "0")}-${String(d.getDate()).padStart(2, "0")}`;

function addDays(day: string, n: number): string {
  const [y = 0, m = 1, d = 1] = day.split("-").map(Number);
  return iso(new Date(y, m - 1, d + n));
}

/** Reads pasted depth: one level per line, price then size, split by spaces, tabs,
 * commas or semicolons. Returns the levels, or the first line that does not read. */
export function parseLevels(text: string): { levels: [string, string][] } | { bad: string } {
  const levels: [string, string][] = [];
  for (const raw of text.split(/\r?\n/)) {
    const line = raw.trim();
    if (!line) continue;
    const parts = line.split(/[\s,;]+/).filter(Boolean);
    const [p, s] = parts;
    if (parts.length !== 2 || p === undefined || s === undefined) return { bad: line };
    levels.push([p, s]);
  }
  return { levels };
}

core.init();
export const meta: Meta = core.meta();

/** The Kalshi taker formula the PRD states. venue-specs marks Kalshi's model fee
 * unverified, so this is the user's own input: prefilled, shown and editable. */
export const PRD_KALSHI_TAKER = "0.07 * C * P * (1 - P)";

export function newLeg(venue: Venue, side: Side, price: string, category: string, payout: string): LegState {
  return {
    venue,
    side,
    role: "taker",
    depth: false,
    price,
    size: "100",
    levels: "",
    category,
    modelFee: { taker: venue === "kalshi" ? PRD_KALSHI_TAKER : "", maker: "" },
    payoutDate: payout,
  };
}

export const today = iso(new Date());

type Listener = () => void;

class Store {
  // Opens on the PRD's first acceptance case (Kalshi YES 0.41 + Polymarket NO 0.55,
  // economics, 100 pairs, paying out in 45 days), so the page shows a worked check.
  st: State = {
    event: "",
    legs: [
      newLeg("kalshi", "yes", "0.41", "", addDays(today, 45)),
      newLeg("polymarket", "no", "0.55", "economics", addDays(today, 45)),
    ],
    fixedCosts: "",
    rulesDate: today,
  };
  result: Result | null = null;
  private listeners: Listener[] = [];
  private timer = 0;

  subscribe(f: Listener): void {
    this.listeners.push(f);
  }

  /** Re-runs the check, at once or after a short pause in typing. */
  run(now = false): void {
    window.clearTimeout(this.timer);
    const go = (): void => {
      this.result = this.compute();
      for (const f of this.listeners) f();
    };
    if (now) go();
    else this.timer = window.setTimeout(go, 200);
  }

  input(): Input | { err: string } {
    const legs: InputLeg[] = [];
    for (const [i, l] of this.st.legs.entries()) {
      const name = i === 0 ? "Leg A" : "Leg B";
      const v = meta.venues[l.venue];
      const out: InputLeg = { venue: l.venue, side: l.side, role: l.role };
      if (l.depth) {
        const p = parseLevels(l.levels);
        if ("bad" in p) return { err: `${name}: each depth line is a price and a size, not "${p.bad}"` };
        out.levels = p.levels;
      } else {
        out.price = l.price.trim();
        out.size = l.size.trim();
      }
      if (v.categories.length) out.category = l.category;
      if (v.model_fee_unverified[l.role] !== null) out.model_fee = l.modelFee[l.role];
      if (l.payoutDate) out.payout_date = l.payoutDate;
      legs.push(out);
    }
    const fixed = this.st.fixedCosts
      .split(/[,;+]/)
      .map((s) => s.trim().replace(/^\$/, ""))
      .filter(Boolean);
    return {
      event: this.st.event,
      legs: legs as [InputLeg, InputLeg],
      fixed_costs: fixed,
      rules_date: this.st.rulesDate,
    };
  }

  private compute(): Result {
    const input = this.input();
    return "err" in input ? input : core.check(input);
  }
}

export const store = new Store();
