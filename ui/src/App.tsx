import { useEffect, useState } from "react";
import { backend, type ContributionNode, type Series } from "./api";
import { Chart } from "./components/Chart";
import { CountyMap } from "./components/CountyMap";
import { WhyTree } from "./components/WhyTree";

const N_CHARTS = 40;
const MONTHS = 600; // 50 years

// Spike 8 UI slice: 40 charts × 600 months, a county map, a nested "why" tree.
export default function App() {
  const [series, setSeries] = useState<Series[]>([]);
  const [counties, setCounties] = useState<Record<string, number>>({});
  const [why, setWhy] = useState<ContributionNode | null>(null);
  const [timing, setTiming] = useState<string>("");

  useEffect(() => {
    const t0 = performance.now();
    Promise.all([backend.runSeries(N_CHARTS, MONTHS), backend.countyValues("unemployment"), backend.explain("inflation")]).then(
      ([s, c, w]) => {
        const t1 = performance.now();
        setSeries(s);
        setCounties(c);
        setWhy(w);
        requestAnimationFrame(() => {
          const t2 = performance.now();
          const msg = `backend=${backend.kind} data=${(t1 - t0).toFixed(1)}ms render=${(t2 - t1).toFixed(1)}ms`;
          setTiming(msg);
          (window as unknown as { __econTiming: string }).__econTiming = msg;
        });
      },
    );
  }, []);

  return (
    <main style={{ fontFamily: "system-ui, sans-serif", padding: 12 }}>
      <h1 style={{ fontSize: 18 }}>econ-sim — UI spike</h1>
      <p style={{ fontSize: 12, color: "#555" }} id="timing">{timing}</p>
      <section style={{ display: "flex", gap: 24, flexWrap: "wrap" }}>
        <CountyMap values={counties} />
        <div style={{ minWidth: 320 }}>
          <h2 style={{ fontSize: 15 }}>Why did inflation change?</h2>
          {why ? <WhyTree node={why} /> : null}
        </div>
      </section>
      <section style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, 300px)", gap: 8 }}>
        {series.map((s) => (
          <Chart key={s.name} series={s} />
        ))}
      </section>
    </main>
  );
}
