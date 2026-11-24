// Display only: every figure is the core's exact decimal, shown as text.

/** The digits of a JSON number from the core (its shortest round-trip form). */
function digits(x: number): string {
  const s = String(x);
  return /e/i.test(s) ? x.toFixed(12).replace(/0+$/, "").replace(/\.$/, "") : s;
}

/** Dollars with at least two decimals and every decimal the core gave: -$1.00, $2.0525. */
export function money(x: number): string {
  const [i = "0", f = ""] = digits(Math.abs(x)).split(".");
  const sign = x < 0 ? "-" : "";
  return `${sign}$${Number(i).toLocaleString("en-US")}.${f.padEnd(2, "0")}`;
}

/** A count or price as the core gave it. */
export const num = (x: number): string => digits(x);

/** A price to at most 4 decimals (averages can run long). */
export const price = (x: number): string => String(Number(x.toFixed(4)));

/** A ratio as a percent with two decimals: 0.021 -> 2.10%. */
export const pct = (x: number): string => `${(Math.round(x * 1e6) / 1e4).toFixed(2)}%`;

export const pairs = (n: number): string => `${n.toLocaleString("en-US")} ${n === 1 ? "pair" : "pairs"}`;
