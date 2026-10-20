import { useCallback, useEffect, useRef, useState } from "react";
import { backend, type Series, type WorldStatus } from "./api";
import { Chart } from "./components/Chart";

// The thin UI of roadmap M2: time controls and a dozen monthly charts on the
// real `World` (the Romania scenario under the placeholder law, no levers).
const STEPS: [string, number][] = [
  ["+1 day", 1],
  ["+1 month", 30],
  ["+1 year", 365],
];
const PLAY_DAYS = 30; // days per frame while playing

export default function App() {
  const [scale, setScale] = useState(1000);
  const [status, setStatus] = useState<WorldStatus | null>(null);
  const [series, setSeries] = useState<Series[]>([]);
  const [busy, setBusy] = useState(false);
  const [playing, setPlaying] = useState(false);
  const [error, setError] = useState("");
  const playRef = useRef(false);

  const run = useCallback(async (job: () => Promise<WorldStatus>) => {
    setBusy(true);
    setError("");
    try {
      const s = await job();
      setStatus(s);
      setSeries(await backend.worldSeries());
      return s;
    } catch (e) {
      setError(String(e));
      return null;
    } finally {
      setBusy(false);
    }
  }, []);

  const open = () => {
    playRef.current = false;
    setPlaying(false);
    setSeries([]);
    void run(() => backend.worldOpen(scale));
  };
  const step = (days: number) => run(() => backend.worldStep(days, scale));

  // Play: one step after another until paused or a check fails.
  useEffect(() => {
    playRef.current = playing;
    if (!playing) return;
    let alive = true;
    (async () => {
      while (alive && playRef.current) {
        const s = await step(PLAY_DAYS);
        if (!s || s.failed.length) {
          setPlaying(false);
          break;
        }
      }
    })();
    return () => {
      alive = false;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [playing]);

  const isOpen = status?.open ?? false;
  return (
    <main style={{ fontFamily: "system-ui, sans-serif", padding: 12 }}>
      <h1 style={{ fontSize: 18, margin: "0 0 8px" }}>econ-sim</h1>
      <section style={{ display: "flex", gap: 8, alignItems: "center", flexWrap: "wrap", marginBottom: 8 }}>
        <label>
          Scale 1:
          <select value={scale} onChange={(e) => setScale(Number(e.target.value))} disabled={busy || playing}>
            {[1000, 100].map((s) => (
              <option key={s} value={s}>
                {s}
              </option>
            ))}
          </select>
        </label>
        <button onClick={open} disabled={busy || playing}>
          {isOpen ? "Restart" : "Open Romania 2021"}
        </button>
        {STEPS.map(([label, days]) => (
          <button key={label} onClick={() => void step(days)} disabled={!isOpen || busy || playing}>
            {label}
          </button>
        ))}
        <button onClick={() => setPlaying((p) => !p)} disabled={!isOpen}>
          {playing ? "Pause" : "Play"}
        </button>
        <span style={{ fontSize: 13, color: "#444" }}>
          {isOpen
            ? `${status!.date} · day ${status!.day} · ${status!.months} months closed · 1:${status!.sample_scale}`
            : "No world open"}
          {busy && !playing ? " · working…" : ""}
          {` · backend=${backend.kind}`}
        </span>
      </section>
      {status?.failed.length ? (
        <p style={{ color: "#b00020", fontSize: 13 }}>Checks failed on day {status.day}: {status.failed.join(", ")}</p>
      ) : null}
      {error ? <p style={{ color: "#b00020", fontSize: 13 }}>{error}</p> : null}
      <section style={{ display: "grid", gridTemplateColumns: "repeat(auto-fill, 320px)", gap: 8 }}>
        {series.map((s) => (
          <Chart key={s.name} series={s} width={320} height={160} />
        ))}
      </section>
      {isOpen && series.length && series[0].values.length === 0 ? (
        <p style={{ fontSize: 13, color: "#555" }}>The charts start at the first monthly close.</p>
      ) : null}
    </main>
  );
}
