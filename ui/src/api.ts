// The only file that knows how the UI talks to the simulation core (ADR-0013).
// In the Tauri app it calls Rust commands; in a plain browser (`npm run dev`,
// perf tests) it uses a deterministic mock with the same data shapes.

import { invoke, isTauri } from "@tauri-apps/api/core";

export interface Series {
  name: string;
  values: Float64Array; // one value per month
}

export interface ContributionNode {
  label: string;
  value: number;
  children: ContributionNode[];
}

export interface Backend {
  kind: "tauri" | "mock";
  /** `nSeries` monthly series of length `months` (binary transfer in Tauri). */
  runSeries(nSeries: number, months: number): Promise<Series[]>;
  /** One value per county code (e.g. "RO-CJ"). */
  countyValues(indicator: string): Promise<Record<string, number>>;
  /** Why did `indicator` change between two months? */
  explain(indicator: string): Promise<ContributionNode>;
}

function unpack(buf: ArrayBuffer, nSeries: number, months: number): Series[] {
  const all = new Float64Array(buf);
  return Array.from({ length: nSeries }, (_, s) => ({
    name: `series ${s + 1}`,
    values: all.subarray(s * months, (s + 1) * months),
  }));
}

const tauriBackend: Backend = {
  kind: "tauri",
  async runSeries(nSeries, months) {
    const buf = await invoke<ArrayBuffer>("run_series", { nSeries, months });
    return unpack(buf, nSeries, months);
  },
  countyValues: (indicator) => invoke<Record<string, number>>("county_values", { indicator }),
  explain: (indicator) => invoke<ContributionNode>("explain", { indicator }),
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

const mockBackend: Backend = {
  kind: "mock",
  async runSeries(nSeries, months) {
    const buf = new Float64Array(nSeries * months);
    for (let s = 0; s < nSeries; s++) {
      const r = mulberry32(s + 1);
      let v = 100;
      for (let m = 0; m < months; m++) {
        v *= 1 + 0.002 + (r() - 0.5) * 0.02;
        buf[s * months + m] = v;
      }
    }
    return unpack(buf.buffer, nSeries, months);
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
};

export const backend: Backend = isTauri() ? tauriBackend : mockBackend;
