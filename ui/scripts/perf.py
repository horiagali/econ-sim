"""Measure the UI slice in headless Chromium (WebView2 is Chromium-based too).

Usage (from ui/): npm run build && npx vite preview --port 4173 &
                  python scripts/perf.py
"""
import sys
import time

from playwright.sync_api import sync_playwright

URL = sys.argv[1] if len(sys.argv) > 1 else "http://localhost:4173/"
with sync_playwright() as p:
    b = p.chromium.launch()
    page = b.new_page(viewport={"width": 1600, "height": 1200}, locale="en-US")
    t0 = time.perf_counter()
    page.goto(URL)
    page.wait_for_function("window.__econTiming !== undefined", timeout=30000)
    total = (time.perf_counter() - t0) * 1000
    timing = page.evaluate("window.__econTiming")
    charts = page.evaluate("document.querySelectorAll('.uplot').length")
    mem = page.evaluate("performance.memory ? performance.memory.usedJSHeapSize : 0")
    print(f"{timing} charts={charts} page_ready_total={total:.0f}ms js_heap={mem/1e6:.1f}MB")
    b.close()
