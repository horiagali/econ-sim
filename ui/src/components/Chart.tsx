import { useEffect, useRef } from "react";
import uPlot from "uplot";
import "uplot/dist/uPlot.min.css";
import type { Series } from "../api";

// One time-series chart (uPlot: fast canvas rendering of large series).
export function Chart({ series, width = 300, height = 140 }: { series: Series; width?: number; height?: number }) {
  const ref = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!ref.current) return;
    const x = Float64Array.from(series.values, (_, i) => i);
    const plot = new uPlot(
      {
        width,
        height,
        title: series.name,
        legend: { show: false },
        scales: { x: { time: false } },
        axes: [{ size: 24 }, { size: 40 }],
        series: [{}, { stroke: "#3b6fd8", width: 1 }],
      },
      [x, series.values] as unknown as uPlot.AlignedData,
      ref.current,
    );
    return () => plot.destroy();
  }, [series, width, height]);
  return <div ref={ref} />;
}
