// The only file that knows how the UI talks to the simulation core (ADR-0013).
// In the Tauri app it calls Rust commands; in a plain browser (`npm run dev`,
// perf tests) it uses a deterministic mock with the same data shapes.

import { invoke, isTauri } from "@tauri-apps/api/core";

import type { Answer, Command, Query } from "./generated/api";

export type * from "./generated/api";

export interface Series {
  name: string;
  values: Float64Array; // one value per month
}

export interface ContributionNode {
  label: string;
  value: number;
  children: ContributionNode[];
}

/** Where the open world is (see `src-tauri/src/world.rs`). */
export interface WorldStatus {
  open: boolean;
  sample_scale: number;
  day: number;
  date: string;
  months: number;
  /** The checks of the latest day that failed; empty when all hold. */
  failed: string[];
}

export interface Backend {
  kind: "tauri" | "mock";
  /** Build the Romania scenario at 1:`sampleScale` and open a world on it. */
  worldOpen(sampleScale: number): Promise<WorldStatus>;
  /** Run `days` days with no command (stops at a day whose checks fail). */
  worldStep(days: number, sampleScale: number): Promise<WorldStatus>;
  /** The world's monthly series, one value per closed month (binary transfer in Tauri). */
  worldSeries(): Promise<Series[]>;
  /** One value per county code (e.g. "RO-CJ"). */
  countyValues(indicator: string): Promise<Record<string, number>>;
  /** Why did `indicator` change between two months? */
  explain(indicator: string): Promise<ContributionNode>;
  /** A query of the core API (spec game/core-api): reads, never changes the world. */
  query(q: Query): Promise<Answer>;
  /** A command of the core API: steps, orders, watches; answers what it did. */
  command(c: Command): Promise<Answer>;
}

function unpack(buf: ArrayBuffer, names: string[]): Series[] {
  const all = new Float64Array(buf);
  const months = names.length ? all.length / names.length : 0;
  return names.map((name, s) => ({ name, values: all.subarray(s * months, (s + 1) * months) }));
}

let seriesNames: string[] | null = null;

const tauriBackend: Backend = {
  kind: "tauri",
  worldOpen: (sampleScale) => invoke<WorldStatus>("world_open", { sampleScale }),
  worldStep: (days, sampleScale) => invoke<WorldStatus>("world_step", { days, sampleScale }),
  async worldSeries() {
    seriesNames ??= await invoke<string[]>("world_series_names");
    const buf = await invoke<ArrayBuffer>("world_series");
    return unpack(buf, seriesNames);
  },
  countyValues: (indicator) => invoke<Record<string, number>>("county_values", { indicator }),
  explain: (indicator) => invoke<ContributionNode>("explain", { indicator }),
  query: (q) => invoke<Answer>("api_query", { q }),
  command: (c) => invoke<Answer>("api_command", { c }),
};

// Deterministic pseudo-random walk (same numbers every run).
function mulberry32(seed: number) {
  return () => {
    seed |= 0;
    seed = (seed + 0x6d2b79f5) | 0;
    let t = Math.imul(seed ^ (seed >>> 15), 1 | seed);
    t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
    return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
  };
}

export const COUNTIES = [
  "AB", "AR", "AG", "BC", "BH", "BN", "BT", "BV", "BR", "B", "BZ", "CS", "CL", "CJ", "CT", "CV",
  "DB", "DJ", "GL", "GR", "GJ", "HR", "HD", "IL", "IS", "IF", "MM", "MH", "MS", "NT", "OT", "PH",
  "SM", "SJ", "SB", "SV", "TR", "TM", "TL", "VS", "VL", "VN",
];

// The mock world: a day counter and random walks, for the browser only.
const MOCK_SERIES = [
  "Unemployment rate (%)", "Inflation, 12 months (%)", "Expected inflation (%)", "CPI (start = 100)",
  "Employed (thousands)", "GDP, production (bn lei)", "Household consumption (bn lei)", "Wage bill (bn lei)",
  "Wage share of GDP (%)", "Consumption share of GDP (%)", "Shortages (bn lei)", "Households in hardship (thousands)",
];
let mockDay = -1;
let mockScale = 1000;

function mockStatus(): WorldStatus {
  const open = mockDay >= 0;
  const day = Math.max(mockDay, 0);
  return {
    open,
    sample_scale: mockScale,
    day,
    date: open ? new Date(Date.UTC(2021, 11, 1 + day)).toISOString().slice(0, 10) : "",
    months: open ? Math.floor(day / 30.4) : 0,
    failed: [],
  };
}

const mockBackend: Backend = {
  kind: "mock",
  async worldOpen(sampleScale) {
    mockDay = 0;
    mockScale = sampleScale;
    return mockStatus();
  },
  async worldStep(days) {
    if (mockDay < 0) throw new Error("no world is open");
    mockDay += days;
    return mockStatus();
  },
  async worldSeries() {
    const months = mockStatus().months;
    return MOCK_SERIES.map((name, s) => {
      const r = mulberry32(s + 1);
      const values = new Float64Array(months);
      let v = 10 + 5 * s;
      for (let m = 0; m < months; m++) {
        v *= 1 + (r() - 0.5) * 0.02;
        values[m] = v;
      }
      return { name, values };
    });
  },
  async countyValues() {
    const r = mulberry32(7);
    return Object.fromEntries(COUNTIES.map((c) => [`RO-${c}`, 3 + 9 * r()]));
  },
  async explain(indicator) {
    return {
      label: `${indicator}: +1.9 pp`,
      value: 1.9,
      children: [
        { label: "energy prices", value: 0.9, children: [
          { label: "gas wholesale price", value: 0.6, children: [] },
          { label: "carbon price (ETS)", value: 0.3, children: [] },
        ] },
        { label: "wages (unit labour cost)", value: 0.7, children: [] },
        { label: "VAT change", value: 0.5, children: [] },
        { label: "exchange rate (EUR/RON)", value: -0.2, children: [] },
      ],
    };
  },
  async query(q) {
    if (q === "Status") {
      const s = mockStatus();
      const [year, month, day] = s.date ? s.date.split("-").map(Number) : [2021, 12, 1];
      return {
        Status: {
          date: { year, month, day },
          day: s.day,
          month: s.months,
          sample_scale: s.sample_scale,
          law: "mock",
          features: [],
        },
      };
    }
    throw new Error("the browser mock answers only the Status query");
  },
  async command(c) {
    if (typeof c === "object" && "Step" in c) {
      const before = mockStatus();
      await mockBackend.worldStep(c.Step.days, mockScale);
      const after = mockStatus();
      return { Stepped: { days: after.day - before.day, closes: after.months - before.months } };
    }
    throw new Error("the browser mock carries out only the Step command");
  },
};

export const backend: Backend = isTauri() ? tauriBackend : mockBackend;
