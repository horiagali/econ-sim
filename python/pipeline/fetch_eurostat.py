#!/usr/bin/env python3
"""Fetch the datasets in sources.toml with provenance: from the Eurostat API (JSON-stat), or from
INS's TEMPO database (`source = "ins_tempo"`, CSV text of a matrix's pivot query)."""
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
TEMPO = "http://statistici.insse.ro:8077/tempo-ins/"


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


def tempo_query(meta: dict, select: dict[str, list[str]]) -> str:
    """The pivot query of a TEMPO matrix: for each dimension, the item ids whose labels are listed
    under a key the dimension's label starts with, or every item if no key matches."""
    parts = []
    for dim in meta["dimensionsMap"]:
        opts = dim["options"]
        key = next((k for k in select if dim["label"].startswith(k)), None)
        ids = [o["nomItemId"] for o in opts if key is None or o["label"].strip() in select[key]]
        if not ids:
            raise ValueError(f"{meta.get('matrixName')}: nothing selected in {dim['label']}")
        parts.append(",".join(str(i) for i in ids))
    return ":".join(parts)


def fetch_tempo(ds: dict) -> tuple[str, bytes]:
    """A TEMPO matrix (INS's database), as the CSV text of its pivot query."""
    code = ds["code"]
    meta = json.loads(download(TEMPO + "matrix/" + code))
    query = tempo_query(meta, ds.get("select", {}))
    body = json.dumps({"language": "ro", "encQuery": query, "matCode": code,
                       "matMaxDim": len(meta["dimensionsMap"]), "matUMSpec": 0, "matRegJ": 0}).encode()
    req = urllib.request.Request(TEMPO + "pivot", data=body, headers={"Content-Type": "application/json"})
    with urllib.request.urlopen(req, timeout=60) as r:  # noqa: S310 (fixed INS host)
        return f"{TEMPO}pivot?matCode={code}&encQuery={query}", r.read()


def fetch(ds: dict) -> Path:
    if ds["source"] == "ins_tempo":
        url, body = fetch_tempo(ds)
        ext = "csv"
    else:
        url = url_for(ds)
        body = download(url)
        ext = "json"
    RAW.mkdir(parents=True, exist_ok=True)
    out = RAW / f"{ds['id']}.{ext}"
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
