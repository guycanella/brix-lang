#!/usr/bin/env python3
"""Minimal mock of a Chroma server (HTTP API v2) for the vectordb tests.

Listens on 127.0.0.1 on a free port and prints the port on the first line of
stdout. Responses follow a real `chroma run` 1.x server: the same shapes,
status codes and error bodies, insertion-ordered get/list, cosine distances,
upserts that merge metadata (a key set to null removes it) and metadata
values limited to scalars or single-type scalar lists (else 422). POSTs must
send a JSON content type, GETs none. Collections (all of dimension 3):
  default_tenant/default_database/docs  -> a, b, c (cosine)
  acme/prod/docs                        -> z only, to tell the paths apart
Auth: no token is accepted (an open local server); a token must be
"test-key" in both x-chroma-token and Authorization: Bearer, else 401.
Like Chroma Cloud, a write of more than 300 records is rejected, so the
batching tests prove the splitting. Id "reject-me" fails its request (a test
hook for a partially written batch). State lives in this process.
"""
import json
import math
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

DIMENSION = 3
MAX_BATCH = 300
PREFIX = "/api/v2/tenants/"

# (tenant, database, name) -> {"id": uuid, "records": {id: (values, metadata)}}
COLLECTIONS = {
    ("default_tenant", "default_database", "docs"): {
        "id": "0b6b9a2e-0000-4000-8000-000000000001",
        "records": {
            "a": ([1.0, 0.0, 0.0], {"genre": "news"}),
            "b": ([0.6, 0.8, 0.0], {"genre": "blog"}),
            "c": ([0.0, 1.0, 0.0], None),
        },
    },
    ("acme", "prod", "docs"): {
        "id": "0b6b9a2e-0000-4000-8000-000000000002",
        "records": {"z": ([0.0, 0.0, 1.0], None)},
    },
}


def cosine_distance(u, v):
    nu = math.sqrt(sum(x * x for x in u))
    nv = math.sqrt(sum(x * x for x in v))
    if nu == 0 or nv == 0:
        return 1.0
    return 1.0 - sum(x * y for x, y in zip(u, v)) / (nu * nv)


def valid_value(v):
    """A metadata value real Chroma accepts: a scalar (or null, which removes
    the key), or a list of scalars of one type; objects, mixed or nested
    lists fail its deserialization with 422."""
    scalar = (str, int, float, bool)
    if v is None or isinstance(v, scalar):
        return True
    if isinstance(v, list):
        kinds = {type(x) for x in v}
        return all(isinstance(x, scalar) for x in v) and len(kinds) <= 1
    return False


def matches_where(meta, where):
    for key, cond in where.items():
        want = cond["$eq"] if isinstance(cond, dict) else cond
        if (meta or {}).get(key) != want:
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

    def error(self, status, kind, message):
        self.reply(status, {"error": kind, "message": message})

    def authorized(self):
        token = self.headers.get("x-chroma-token")
        bearer = self.headers.get("Authorization")
        if token is None and bearer is None:
            return True
        if token != "test-key" or bearer != "Bearer test-key":
            self.error(401, "AuthError", "Unauthorized")
            return False
        return True

    def route(self):
        """(collection, op) for the request path; op is "" for the
        collection itself. Replies 404 and returns None when unknown."""
        parts = self.path.split("/")
        # "", api, v2, tenants, T, databases, D, collections, X[, op]
        if (not self.path.startswith(PREFIX) or len(parts) not in (9, 10)
                or parts[5] != "databases" or parts[7] != "collections"):
            self.error(404, "NotFoundError", "Not Found")
            return None
        tenant, database, ref = parts[4], parts[6], parts[8]
        op = parts[9] if len(parts) == 10 else ""
        for (t, d, name), coll in COLLECTIONS.items():
            if (t, d) != (tenant, database):
                continue
            # The collection itself is looked up by name, operations by id.
            if (op == "" and ref == name) or (op and ref == coll["id"]):
                return coll, op
        self.error(404, "NotFoundError", f"Collection [{ref}] does not exist")
        return None

    def do_GET(self):
        if not self.authorized():
            return
        if self.headers.get("Content-Type"):
            return self.error(400, "InvalidArgumentError", "GET must not send a Content-Type")
        found = self.route()
        if found is None:
            return
        coll, op = found
        if op == "":
            return self.reply(200, {"id": coll["id"], "dimension": DIMENSION,
                                    "metadata": {"hnsw:space": "cosine"}})
        if op == "count":
            return self.reply(200, len(coll["records"]))
        return self.error(404, "NotFoundError", "Not Found")

    def do_POST(self):
        raw = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if not self.authorized():
            return
        if self.headers.get("Content-Type") != "application/json":
            return self.error(415, "ChromaError", "Content-Type must be application/json")
        try:
            body = json.loads(raw)
        except ValueError:
            return self.error(400, "ChromaError", "body is not JSON")
        found = self.route()
        if found is None:
            return
        coll, op = found
        handler = {"query": self.query, "get": self.get, "upsert": self.upsert,
                   "delete": self.delete}.get(op)
        if handler is None:
            return self.error(404, "NotFoundError", "Not Found")
        return handler(coll["records"], body)

    def query(self, records, body):
        (vec,) = body["query_embeddings"]
        if len(vec) != DIMENSION:
            return self.error(400, "InvalidArgumentError",
                              f"Collection expecting embedding with dimension of {DIMENSION}, got {len(vec)}")
        where = body.get("where") or {}
        hits = sorted(
            (cosine_distance(vec, values), rid, meta)
            for rid, (values, meta) in records.items()
            if matches_where(meta, where)
        )[:body["n_results"]]
        return self.reply(200, {
            "ids": [[h[1] for h in hits]],
            "distances": [[round(h[0], 6) for h in hits]],
            "metadatas": [[h[2] for h in hits]],
            "embeddings": None, "documents": None, "uris": None,
            "include": body.get("include", []),
        })

    def get(self, records, body):
        include = body.get("include", [])
        ids = [i for i in (body.get("ids") or records) if i in records]
        if body.get("ids") is None:
            start = body.get("offset") or 0
            ids = ids[start:start + body.get("limit", len(ids))]
        return self.reply(200, {
            "ids": ids,
            "embeddings": [records[i][0] for i in ids] if "embeddings" in include else None,
            "metadatas": [records[i][1] for i in ids] if "metadatas" in include else None,
            "documents": None, "uris": None, "include": include,
        })

    def upsert(self, records, body):
        ids, vecs = body["ids"], body["embeddings"]
        metas = body.get("metadatas") or [None] * len(ids)
        if len(ids) > MAX_BATCH:
            return self.error(422, "QuotaError", f"more than {MAX_BATCH} records per write")
        if "reject-me" in ids:
            return self.error(500, "InternalError", "rejected")
        for i, meta in enumerate(metas):
            for k, val in (meta or {}).items():
                if not valid_value(val):
                    return self.error(422, "ChromaError",
                                      "Failed to deserialize the JSON body into the target type: "
                                      f"metadatas[{i}].{k}: data did not match any variant of "
                                      "untagged enum UpdateMetadataValue")
        for v in vecs:
            if len(v) != DIMENSION:
                return self.error(400, "InvalidArgumentError",
                                  f"Collection expecting embedding with dimension of {DIMENSION}, got {len(v)}")
        for rid, values, meta in zip(ids, vecs, metas):
            merged = dict(records[rid][1] or {}) if rid in records else {}
            for k, val in (meta or {}).items():
                if val is None:
                    merged.pop(k, None)
                else:
                    merged[k] = val
            records[rid] = (values, merged or None)
        return self.reply(200, {})

    def delete(self, records, body):
        ids = body.get("ids") or []
        if len(ids) > MAX_BATCH:
            return self.error(422, "QuotaError", f"more than {MAX_BATCH} records per write")
        if "reject-me" in ids:
            return self.error(500, "InternalError", "rejected")
        deleted = sum(records.pop(i, None) is not None for i in ids)
        return self.reply(200, {"deleted": deleted})


def main():
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    print(server.server_address[1], flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())
