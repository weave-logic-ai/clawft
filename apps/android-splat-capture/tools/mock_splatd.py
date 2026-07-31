#!/usr/bin/env python3
"""Minimal splatd stub for A2 (WEFT-706) phone→host smoke without GPU train.

Endpoints:
  GET  /healthz
  POST /v1/jobs          (multipart field `session` → ZIP)
  GET  /v1/jobs
  GET  /v1/jobs/{id}
  GET  /v1/jobs/{id}/artifacts/{name}

Usage:
  python3 apps/android-splat-capture/tools/mock_splatd.py --port 7860
"""

from __future__ import annotations

import argparse
import json
import time
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path
from urllib.parse import urlparse

JOBS: dict[str, dict] = {}
DATA = Path("/tmp/weft-mock-splatd")
DATA.mkdir(parents=True, exist_ok=True)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, fmt: str, *args) -> None:
        print(f"[mock-splatd] {self.address_string()} {fmt % args}")

    def _json(self, code: int, obj: object) -> None:
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("content-type", "application/json")
        self.send_header("content-length", str(len(body)))
        self.send_header("access-control-allow-origin", "*")
        self.end_headers()
        self.wfile.write(body)

    def do_OPTIONS(self) -> None:
        self.send_response(204)
        self.send_header("access-control-allow-origin", "*")
        self.send_header("access-control-allow-methods", "GET, POST, OPTIONS")
        self.send_header("access-control-allow-headers", "content-type, authorization")
        self.end_headers()

    def do_GET(self) -> None:
        path = urlparse(self.path).path
        if path == "/healthz":
            return self._json(
                200,
                {
                    "status": "healthy",
                    "mock": True,
                    "jobs": len(JOBS),
                    "version": "mock-splatd-0.1",
                },
            )
        if path == "/v1/jobs":
            items = [
                {
                    "job_id": j["job_id"],
                    "status": j["status"],
                    "stage": j["stage"],
                    "created_ts_ms": j["created_ts_ms"],
                }
                for j in JOBS.values()
            ]
            return self._json(200, {"jobs": items})
        if path.startswith("/v1/jobs/") and "/artifacts/" in path:
            # /v1/jobs/{id}/artifacts/{name}
            parts = path.strip("/").split("/")
            # v1 jobs id artifacts name
            if len(parts) >= 5:
                jid, name = parts[2], parts[4]
                art = DATA / jid / name
                if art.is_file():
                    data = art.read_bytes()
                    self.send_response(200)
                    self.send_header("content-type", "application/octet-stream")
                    self.send_header("content-length", str(len(data)))
                    self.send_header("access-control-allow-origin", "*")
                    self.end_headers()
                    self.wfile.write(data)
                    return
                return self._json(404, {"error": "no artifact"})
        if path.startswith("/v1/jobs/"):
            jid = path.rstrip("/").split("/")[-1]
            rec = JOBS.get(jid)
            if not rec:
                return self._json(404, {"error": "no such job"})
            # Advance mock pipeline on each poll.
            age = time.time() - rec["t0"]
            if age > 8:
                rec["status"] = "done"
                rec["stage"] = "package"
                rec["artifacts"] = {"scene.sog": True}
            elif age > 4:
                rec["status"] = "running"
                rec["stage"] = "train"
            elif age > 1:
                rec["status"] = "running"
                rec["stage"] = "sfm"
            return self._json(200, rec)
        self._json(404, {"error": "not found"})

    def do_POST(self) -> None:
        path = urlparse(self.path).path
        if path != "/v1/jobs":
            return self._json(404, {"error": "not found"})
        length = int(self.headers.get("content-length", "0"))
        raw = self.rfile.read(length) if length else b""
        jid = str(uuid.uuid4())
        job_dir = DATA / jid
        job_dir.mkdir(parents=True, exist_ok=True)
        # Store raw multipart / body for inspection.
        (job_dir / "upload.bin").write_bytes(raw)
        # Tiny fake SOG artifact.
        (job_dir / "scene.sog").write_bytes(b"SOG-MOCK\n")
        rec = {
            "job_id": jid,
            "status": "queued",
            "stage": "ingest",
            "created_ts_ms": int(time.time() * 1000),
            "artifacts": {},
            "t0": time.time(),
            "source": "session-upload",
        }
        JOBS[jid] = rec
        return self._json(202, {"job_id": jid})


def main() -> None:
    ap = argparse.ArgumentParser()
    ap.add_argument("--host", default="0.0.0.0")
    ap.add_argument("--port", type=int, default=7860)
    args = ap.parse_args()
    httpd = ThreadingHTTPServer((args.host, args.port), Handler)
    print(f"mock splatd on http://{args.host}:{args.port}  (data={DATA})")
    httpd.serve_forever()


if __name__ == "__main__":
    main()
