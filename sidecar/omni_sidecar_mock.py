#!/usr/bin/env python3
import argparse
import json
import time
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer


def json_response(handler, status, payload):
    data = json.dumps(payload).encode("utf-8")
    handler.send_response(status)
    handler.send_header("Content-Type", "application/json; charset=utf-8")
    handler.send_header("Content-Length", str(len(data)))
    handler.end_headers()
    handler.wfile.write(data)


class Handler(BaseHTTPRequestHandler):
    def do_GET(self):
        if self.path in ("/probe", "/probe/"):
            payload = {
                "ok": True,
                "model": "omniparser-mock",
                "gpu": "unknown",
                "message": "Omniparser API ready (mock)",
            }
            json_response(self, 200, payload)
            return
        json_response(self, 404, {"ok": False, "error": "not found"})

    def do_POST(self):
        if self.path in ("/parse", "/parse/"):
            length = int(self.headers.get("Content-Length", "0"))
            if length:
                _ = self.rfile.read(length)
            start = time.time()
            latency_s = time.time() - start
            payload = {
                "latency": latency_s,
                "latency_ms": int(latency_s * 1000),
                "parsed_content_list": [
                    {"type": "text", "content": "mock text", "score": 0.5},
                    {"type": "icon", "content": "mock-icon", "score": 0.2},
                ],
                "som_image_base64": "",
            }
            json_response(self, 200, payload)
            return
        json_response(self, 404, {"ok": False, "error": "not found"})

    def log_message(self, format, *args):
        return


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--host", default="127.0.0.1")
    parser.add_argument("--port", type=int, default=8000)
    args = parser.parse_args()

    server = ThreadingHTTPServer((args.host, args.port), Handler)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    main()
