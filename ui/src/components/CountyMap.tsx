import { useEffect, useRef, useState } from "react";
import * as echarts from "echarts";

// Choropleth of Romania's 42 counties (ECharts). Boundaries come from
// geoBoundaries (CC BY 4.0) — run `npm run fetch-geo` once to download them.
export function CountyMap({ values }: { values: Record<string, number> }) {
  const ref = useRef<HTMLDivElement>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let chart: echarts.ECharts | null = null;
    fetch("/geo/romania-adm1.geojson")
      .then((r) => {
        if (!r.ok) throw new Error("missing");
        return r.json();
      })
      .then((geo) => {
        if (!ref.current) return;
        echarts.registerMap("romania", geo);
        chart = echarts.init(ref.current);
        const data = Object.entries(values).map(([name, value]) => ({ name, value }));
        chart.setOption({
          tooltip: { trigger: "item" },
          visualMap: { min: 0, max: 12, left: "left", text: ["high", "low"], calculable: true },
          series: [{ type: "map", map: "romania", nameProperty: "shapeISO", data }],
        });
      })
      .catch(() => setError("County boundaries not found. Run `npm run fetch-geo` (needs internet)."));
    return () => chart?.dispose();
  }, [values]);
  return (
    <div>
      {error ? <p style={{ color: "#a33" }}>{error}</p> : null}
      <div ref={ref} style={{ width: 520, height: 420 }} />
      <p style={{ fontSize: 11, color: "#666" }}>Boundaries: geoBoundaries (CC BY 4.0).</p>
    </div>
  );
}
