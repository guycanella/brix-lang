#!/usr/bin/env python3
"""Minimal mock of a Pinecone index data-plane host for the vectordb tests.

Listens on 127.0.0.1 on a free port and prints the port on the first line of
stdout. Every request must carry the headers the Brix runtime sends (Api-Key,
X-Pinecone-API-Version, JSON content type) and a JSON body, or it gets a 400
naming what is missing. API keys:
  test-key      -> normal responses
  bad-json-key  -> 200 with a body that is not JSON
  anything else -> 401, like Pinecone
"""
import json
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

VALID_KEYS = {"test-key", "bad-json-key"}

STATS = {
    "namespaces": {"": {"vectorCount": 3}, "prod": {"vectorCount": 2}},
    "dimension": 3,
    "indexFullness": 0.0,
    "totalVectorCount": 5,
}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, status, body):
        data = body if isinstance(body, bytes) else json.dumps(body).encode()
        self.send_response(status)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length)
        key = self.headers.get("Api-Key")
        if key not in VALID_KEYS:
            return self.reply(401, {"code": 16, "message": "Invalid API Key"})
        if not self.headers.get("X-Pinecone-API-Version"):
            return self.reply(400, {"message": "missing X-Pinecone-API-Version"})
        if self.headers.get("Content-Type") != "application/json":
            return self.reply(400, {"message": "Content-Type must be application/json"})
        try:
            json.loads(raw)
        except ValueError:
            return self.reply(400, {"message": "body is not JSON"})
        if key == "bad-json-key":
            return self.reply(200, b"<html>not json</html>")
        if self.path == "/describe_index_stats":
            return self.reply(200, STATS)
        return self.reply(404, {"message": "Not Found"})


def main():
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    print(server.server_address[1], flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())
