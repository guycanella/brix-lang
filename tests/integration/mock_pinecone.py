#!/usr/bin/env python3
"""Minimal mock of a Pinecone index data-plane host for the vectordb tests.

Listens on 127.0.0.1 on a free port and prints the port on the first line of
stdout. Every request must carry the headers the Brix runtime sends (Api-Key,
X-Pinecone-API-Version; POSTs also a JSON content type and a JSON body), or it
gets a 400 naming what is missing. Endpoints: POST /describe_index_stats,
POST /query (cosine, equality filters), GET /vectors/fetch, GET /vectors/list
(paginated). API keys:
  test-key      -> normal responses
  bad-json-key  -> 200 with a body that is not JSON
  anything else -> 401, like Pinecone
"""
import json
import math
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit

VALID_KEYS = {"test-key", "bad-json-key"}

STATS = {
    "namespaces": {"": {"vectorCount": 3}, "prod": {"vectorCount": 2}},
    "dimension": 3,
    "indexFullness": 0.0,
    "totalVectorCount": 5,
}

# namespace -> id -> (values, metadata); dimension 3, matching STATS.
VECTORS = {
    "": {
        "a": ([1.0, 0.0, 0.0], {"genre": "news", "year": 2024}),
        "b": ([0.6, 0.8, 0.0], {"genre": "blog", "year": 2023}),
        "c": ([0.0, 1.0, 0.0], {"genre": "news", "year": 2023}),
    },
    "prod": {
        "p1": ([0.0, 0.0, 1.0], {"env": "prod"}),
        "x&y z": ([1.0, 1.0, 0.0], {"env": "prod"}),
    },
}


def cosine(u, v):
    nu = math.sqrt(sum(x * x for x in u))
    nv = math.sqrt(sum(x * x for x in v))
    if nu == 0 or nv == 0:
        return 0.0
    return sum(x * y for x, y in zip(u, v)) / (nu * nv)


def matches_filter(meta, flt):
    for key, cond in flt.items():
        want = cond["$eq"] if isinstance(cond, dict) else cond
        if meta.get(key) != want:
            return False
    return True


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

    def authorized(self):
        key = self.headers.get("Api-Key")
        if key not in VALID_KEYS:
            self.reply(401, {"code": 16, "message": "Invalid API Key"})
            return None
        if not self.headers.get("X-Pinecone-API-Version"):
            self.reply(400, {"message": "missing X-Pinecone-API-Version"})
            return None
        return key

    def do_POST(self):
        length = int(self.headers.get("Content-Length", 0))
        raw = self.rfile.read(length)
        key = self.authorized()
        if key is None:
            return
        if self.headers.get("Content-Type") != "application/json":
            return self.reply(400, {"message": "Content-Type must be application/json"})
        try:
            body = json.loads(raw)
        except ValueError:
            return self.reply(400, {"message": "body is not JSON"})
        if key == "bad-json-key":
            return self.reply(200, b"<html>not json</html>")
        if self.path == "/describe_index_stats":
            return self.reply(200, STATS)
        if self.path == "/query":
            return self.query(body)
        return self.reply(404, {"message": "Not Found"})

    def query(self, body):
        vec = body.get("vector")
        top_k = body.get("topK")
        if not isinstance(top_k, int) or top_k < 1:
            return self.reply(400, {"message": "topK must be a positive integer"})
        if not isinstance(vec, list) or len(vec) != STATS["dimension"]:
            n = len(vec) if isinstance(vec, list) else 0
            return self.reply(400, {"message": f"Vector dimension {n} does not match "
                                               f"the dimension of the index {STATS['dimension']}"})
        flt = body.get("filter") or {}
        records = VECTORS.get(body.get("namespace", ""), {})
        hits = [
            (cosine(vec, values), rid, values, meta)
            for rid, (values, meta) in records.items()
            if matches_filter(meta, flt)
        ]
        hits.sort(key=lambda h: (-h[0], h[1]))
        out = []
        for score, rid, values, meta in hits[:top_k]:
            m = {"id": rid, "score": round(score, 6)}
            if body.get("includeValues"):
                m["values"] = values
            if body.get("includeMetadata"):
                m["metadata"] = meta
            out.append(m)
        return self.reply(200, {"matches": out, "namespace": body.get("namespace", "")})

    def do_GET(self):
        key = self.authorized()
        if key is None:
            return
        if self.headers.get("Content-Type"):
            return self.reply(400, {"message": "GET must not send a Content-Type"})
        url = urlsplit(self.path)
        params = parse_qs(url.query, keep_blank_values=True)
        ns = params.get("namespace", [""])[0]
        records = VECTORS.get(ns, {})
        if url.path == "/vectors/fetch":
            ids = params.get("ids", [])
            if not ids:
                return self.reply(400, {"message": "ids is required"})
            found = {
                i: {"id": i, "values": records[i][0], "metadata": records[i][1]}
                for i in ids
                if i in records
            }
            return self.reply(200, {"vectors": found, "namespace": ns})
        if url.path == "/vectors/list":
            limit = int(params.get("limit", ["100"])[0])
            if limit < 1 or limit > 100:
                return self.reply(400, {"message": "limit must be between 1 and 100"})
            start = int(params.get("paginationToken", ["0"])[0] or "0")
            ids = sorted(records)
            page = ids[start:start + limit]
            resp = {"vectors": [{"id": i} for i in page], "namespace": ns}
            if start + limit < len(ids):
                resp["pagination"] = {"next": str(start + limit)}
            return self.reply(200, resp)
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
