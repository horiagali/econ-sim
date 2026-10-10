#!/usr/bin/env python3
"""Fetch the datasets in sources.toml from the Eurostat API with provenance."""
from __future__ import annotations

import hashlib
import json
import ssl
import subprocess
import sys
import tomllib
import urllib.error
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from pathlib import Path

HERE = Path(__file__).resolve().parent
RAW = HERE / "data" / "raw"
API = "https://ec.europa.eu/eurostat/api/dissemination/statistics/1.0/data/"


def url_for(ds: dict) -> str:
    params = [("format", "JSON"), ("lang", "EN")]
    for k, vs in ds.get("filters", {}).items():
        for v in vs:
            params.append((k, v))
    return API + ds["code"] + "?" + urllib.parse.urlencode(params)


def _read(url: str) -> bytes:
    with urllib.request.urlopen(url, timeout=60) as r:  # noqa: S310 (fixed https host)
        return r.read()


def download(url: str) -> bytes:
    try:
        return _read(url)
    except urllib.error.URLError as e:
        if sys.platform != "win32" or not isinstance(e.reason, ssl.SSLCertVerificationError):
            raise
    # Windows installs root certificates on first use and Python's ssl only sees
    # the ones already installed. One request through the system TLS stack
    # (curl.exe ships with Windows) makes Windows fetch the missing root; the
    # retry below still verifies the certificate.
    try:
        subprocess.run(["curl.exe", "-s", "-o", "NUL", "--max-time", "60", url], check=False)  # noqa: S603, S607
    except OSError:
        pass
    return _read(url)


def fetch(ds: dict) -> Path:
    url = url_for(ds)
    body = download(url)
    RAW.mkdir(parents=True, exist_ok=True)
    out = RAW / f"{ds['id']}.json"
    out.write_bytes(body)
    prov = {
        "id": ds["id"],
        "source": ds["source"],
        "url": url,
        "retrieved_at": datetime.now(timezone.utc).isoformat(timespec="seconds"),
        "sha256": hashlib.sha256(body).hexdigest(),
        "licence": ds["licence"],
    }
    (RAW / f"{ds['id']}.provenance.json").write_text(json.dumps(prov, indent=2), encoding="utf-8")
    return out


def main() -> int:
    cfg = tomllib.loads((HERE / "sources.toml").read_text(encoding="utf-8"))
    for ds in cfg["dataset"]:
        try:
            p = fetch(ds)
            print(f"OK {ds['id']} -> {p.relative_to(HERE)}")
        except Exception as e:  # noqa: BLE001
            print(f"FAILED {ds['id']}: {e}")
            return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
