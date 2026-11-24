// Shapes of the core's JSON (crates/core/src/api.rs and report.rs).

export type Venue = "kalshi" | "polymarket" | "hyperliquid";
export type Side = "yes" | "no";
export type Role = "taker" | "maker";

/** Numbers cross into the core as their exact text. */
export interface InputLeg {
  venue: Venue;
  side: Side;
  role: Role;
  levels?: [string, string][];
  price?: string;
  size?: string;
  category?: string;
  model_fee?: string;
  payout_date?: string;
}

export interface Input {
  event: string;
  legs: [InputLeg, InputLeg];
  fixed_costs: string[];
  rules_date: string;
}

export interface Meta {
  venue_specs: string;
  venues: Record<Venue, { label: string; categories: string[]; model_fee_unverified: Record<Role, string | null> }>;
}

export interface Outcome {
  payout: number;
  settlement_fees: [number, number];
  profit: number;
}

export interface ReportLeg {
  contracts: number;
  cost: number;
  average_price: number;
  model_fee: number;
  rounding: number;
  entry_fees: number;
  fills: number;
  last_price: number;
  payout_date: string | null;
}

export interface Report {
  event: string;
  verdict: string;
  profitable: boolean;
  pairs: number;
  guaranteed_profit: number;
  breakdown: { label: string; amount: number }[];
  by_outcome: { event_happens: Outcome; event_does_not_happen: Outcome; guaranteed: number };
  legs: [ReportLeg, ReportLeg];
  sizing: {
    max_pairs: number;
    optimal_size: number | null;
    break_even_size: number | null;
    average_price: [number, number];
    last_profitable_pair: { pair: number; prices: [number, number]; marginal_profit: number } | null;
  };
  returns: { capital: number; roc: number; days_to_payout: number; apr: number };
  curve: [number, number][];
  provenance: { venue_specs: string; rules_date: string; legs: unknown[] };
  notes: string[];
}
