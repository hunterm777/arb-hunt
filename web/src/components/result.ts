// The assumption and its risks, the verdict, then the cost breakdown, by-outcome
// table, sizing, returns and notes. Every figure is the core's.
import { h } from "../dom";
import { money, num, pairs, pct, price } from "../format";
import { meta, store } from "../state";
import type { Report } from "../types";

/** The PRD's list. arbcalc states these; it does not score or price them. */
const RISKS: readonly [string, string][] = [
  [
    "Resolution mismatch",
    "the two markets use different sources, times or wording, so one can resolve YES and the other NO. Polymarket stock markets resolve on the official close, for example, while HIP-4 recurring binaries resolve on an interpolated mark price at 06:00 UTC.",
  ],
  ["Legging", "one leg fills and the other moves before it fills."],
  ["Payout timing", "venues pay out on different dates, and the capital stays locked until the later one."],
  [
    "Access",
    "Polymarket's international site is not available to US users, who are directed to Polymarket US. Check that you can trade both venues.",
  ],
  ["Fee changes", "rules can change before resolution. HIP-4 fees are charged at settlement under the rules then in force."],
];

function risks(): HTMLElement {
  return h(
    "section",
    { class: "risks", "aria-label": "Assumption and risks" },
    h("p", {}, h("strong", {}, "This assumes both legs resolve the same way. "), "What can break that:"),
    h("ul", {}, ...RISKS.map(([k, v]) => h("li", {}, h("strong", {}, `${k}: `), v))),
  );
}

function table(head: string[], rows: (string | HTMLElement)[][], cls = ""): HTMLElement {
  return h(
    "div",
    { class: "table-scroll" },
    h(
      "table",
      { class: cls },
      h("thead", {}, h("tr", {}, ...head.map((x) => h("th", {}, x)))),
      h("tbody", {}, ...rows.map((r) => h("tr", {}, ...r.map((c, j) => (j === 0 ? h("th", {}, c) : h("td", {}, c)))))),
    ),
  );
}

const signed = (x: number): HTMLElement => h("span", { class: x < 0 ? "neg" : x > 0 ? "pos" : "" }, money(x));

function panels(r: Report): HTMLElement {
  const legName = (i: 0 | 1): string => `${i === 0 ? "A" : "B"} ${meta.venues[store.st.legs[i].venue].label}`;
  const breakdown = table(
    ["Cost breakdown", `On ${pairs(r.pairs)}`],
    r.breakdown.map((b) => [b.label, signed(b.amount)]),
    "money",
  );
  const o = r.by_outcome;
  // Both legs' settlement fees, shown as one cost (12 significant digits drops float noise).
  const settle = (x: [number, number]): string => money(-Number((x[0] + x[1]).toPrecision(12)));
  const byOutcome = table(
    ["By outcome", "Payout", "Settlement fees", "Net profit"],
    [
      ["Event happens", money(o.event_happens.payout), settle(o.event_happens.settlement_fees), signed(o.event_happens.profit)],
      ["Event does not happen", money(o.event_does_not_happen.payout), settle(o.event_does_not_happen.settlement_fees), signed(o.event_does_not_happen.profit)],
      ["Guaranteed (the lower)", "", "", signed(o.guaranteed)],
    ],
    "money",
  );
  const s = r.sizing;
  const last = s.last_profitable_pair;
  const sizing = table(
    ["Sizing", ""],
    [
      ["Profit-maximising size", s.optimal_size === null ? "None: no size makes money" : pairs(s.optimal_size)],
      ["Break-even size", s.break_even_size === null ? "Not reached in the depth entered" : pairs(s.break_even_size)],
      [`Average price, ${legName(0)}`, price(s.average_price[0])],
      [`Average price, ${legName(1)}`, price(s.average_price[1])],
      ["Last profitable pair", last ? `#${num(last.pair)} at ${num(last.prices[0])} + ${num(last.prices[1])}, adds ${money(last.marginal_profit)}` : "None"],
      ["Depth entered", pairs(s.max_pairs)],
    ],
  );
  const ret = r.returns;
  const returns = table(
    ["Returns", ""],
    [
      ["Capital required", money(ret.capital)],
      ["Return on capital", pct(ret.roc)],
      ["Days to payout", num(ret.days_to_payout)],
      ["APR (simple)", pct(ret.apr)],
    ],
  );
  return h("div", { class: "panels" }, breakdown, byOutcome, sizing, returns);
}

export function resultView(): HTMLElement {
  const verdict = h("p", { class: "verdict", role: "status", "aria-live": "polite" });
  const stamp = h("p", { class: "stamp" });
  const body = h("div", { class: "result-body" });
  const notes = h("ul", { class: "notes" });
  store.subscribe(() => {
    const res = store.result;
    if (!res) return;
    if ("err" in res) {
      verdict.className = "verdict error";
      verdict.textContent = res.err;
      stamp.textContent = "";
      body.replaceChildren();
      notes.replaceChildren();
      return;
    }
    const r = res.ok;
    verdict.className = `verdict ${r.profitable ? "pos" : "neg"}`;
    verdict.textContent = r.verdict;
    stamp.textContent = `Fees and rounding from venue-specs ${r.provenance.venue_specs}, rules as of ${r.provenance.rules_date}.`;
    body.replaceChildren(panels(r));
    notes.replaceChildren(...r.notes.map((n) => h("li", {}, n)));
  });
  return h("section", { class: "result", "aria-label": "Result" }, risks(), verdict, stamp, body, notes);
}
