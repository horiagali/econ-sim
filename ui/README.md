# econ-sim UI (Spike 8)

Tauri 2 desktop app: React + TypeScript frontend, uPlot charts, ECharts
county map; the Rust simulation core runs in-process (ADR-0013).

```powershell
cd ui
npm install
npm run fetch-geo        # once: county boundaries from geoBoundaries (CC BY 4.0)
npm run dev              # browser only, mock data (http://localhost:1420)
npm run tauri dev        # desktop app with the real Rust core (needs Rust + WebView2)
```

The page header shows load timings (`backend=… data=…ms`). In a plain
browser the data comes from a mock with the same shapes (`src/api.ts`); in
the Tauri app it comes from the Rust core (`src-tauri/src/main.rs`).

Measured in headless Chromium on the sandbox (mock data): 40 uPlot charts ×
600 months ready ~330 ms after navigation, ~10 MB JS heap. The Tauri host
could not be compiled in the sandbox (no WebView libraries) — the first
`npm run tauri dev` on Windows is the real test.
