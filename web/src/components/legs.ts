// The two leg cards, then fixed costs and the rules date under them.
import { areaField, dateField, h, segmented, selectField, textField } from "../dom";
import { meta, newLeg, store, type LegState } from "../state";
import type { Role, Side, Venue } from "../types";

const VENUES: readonly (readonly [Venue, string])[] = [
  ["kalshi", meta.venues.kalshi.label],
  ["polymarket", meta.venues.polymarket.label],
  ["hyperliquid", meta.venues.hyperliquid.label],
];

function card(i: 0 | 1): HTMLElement {
  const root = h("section", { class: "card", "aria-label": i === 0 ? "Leg A" : "Leg B" });
  const render = (): void => {
    const l = store.st.legs[i];
    const set = (f: (l: LegState) => void, redraw = false): void => {
      f(l);
      if (redraw) render();
      store.run(redraw);
    };
    const v = meta.venues[l.venue];
    const unverified = v.model_fee_unverified[l.role];
    const fields: HTMLElement[] = [
      selectField("Venue", l.venue, VENUES, (x) =>
        set((s) => {
          s.venue = x;
          s.modelFee = newLeg(x, s.side, "", "", "").modelFee;
          const cats = meta.venues[x].categories;
          if (!cats.includes(s.category)) s.category = cats[0] ?? "";
        }, true),
      ),
      h(
        "div",
        { class: "field" },
        h("span", { class: "flabel" }, "Side"),
        segmented<Side>("Side", l.side, [["yes", "YES"], ["no", "NO"]], (x) => set((s) => (s.side = x), true)),
      ),
      h(
        "div",
        { class: "field" },
        h("span", { class: "flabel" }, "Order"),
        segmented<Role>("Maker or taker", l.role, [["taker", "Taker"], ["maker", "Maker"]], (x) =>
          set((s) => (s.role = x), true),
        ),
      ),
      v.categories.length
        ? selectField(
            "Fee category",
            l.category,
            v.categories.map((c) => [c, c.replace(/_/g, " ")] as const),
            (x) => set((s) => (s.category = x)),
          )
        : h("span", { class: "field" }),
      h(
        "div",
        { class: "field wide" },
        h("span", { class: "flabel" }, "Price"),
        segmented<"top" | "depth">("Price entry", l.depth ? "depth" : "top", [["top", "Top of book"], ["depth", "Depth"]], (x) =>
          set((s) => (s.depth = x === "depth"), true),
        ),
      ),
    ];
    if (l.depth) {
      fields.push(areaField("Levels (price size, one per line)", l.levels, (x) => set((s) => (s.levels = x)), "0.41 300\n0.42 500"));
    } else {
      fields.push(
        textField("Price", l.price, (x) => set((s) => (s.price = x)), { num: true, placeholder: "0.41" }),
        textField("Size (contracts)", l.size, (x) => set((s) => (s.size = x)), { num: true, placeholder: "100" }),
      );
    }
    if (unverified !== null) {
      fields.push(
        h(
          "div",
          { class: "wide model" },
          textField(`Your ${l.role} model fee (C contracts, P price)`, l.modelFee[l.role], (x) => set((s) => (s.modelFee[s.role] = x)), {
            placeholder: "0.07 * C * P * (1 - P)",
          }),
          h("p", { class: "hint" }, `venue-specs has not verified this fee: ${unverified}`),
        ),
      );
    }
    fields.push(dateField("Payout date", l.payoutDate, (x) => set((s) => (s.payoutDate = x))));
    root.replaceChildren(
      h("div", { class: "card-head" }, h("span", { class: "kind" }, i === 0 ? "Leg A" : "Leg B"), h("span", { class: "muted" }, v.label)),
      h("div", { class: "fields" }, ...fields),
    );
  };
  render();
  return root;
}

export function legInputs(): HTMLElement {
  return h(
    "div",
    { class: "inputs" },
    textField("Event", store.st.event, (x) => {
      store.st.event = x;
      store.run();
    }, { placeholder: "Your own label, such as Fed cuts in December" }),
    h("div", { class: "cards" }, card(0), card(1)),
    h(
      "div",
      { class: "bar" },
      textField("Fixed costs ($)", store.st.fixedCosts, (x) => {
        store.st.fixedCosts = x;
        store.run();
      }, { num: true, placeholder: "0.00", hint: "Deposits, withdrawals or transfers; separate several with commas" }),
      dateField("Rules date", store.st.rulesDate, (x) => {
        store.st.rulesDate = x;
        store.run();
      }),
    ),
  );
}
