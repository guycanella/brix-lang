#!/usr/bin/env python3
"""Minimal mock of a Milvus server (REST API v2) for the vectordb tests.

Listens on 127.0.0.1 on a free port and prints the port on the first line of
stdout. Responses follow a real Milvus 2.5 server: every route is a POST under
/v2/vectordb, errors are HTTP 200 with a non-zero "code" (auth: HTTP 401),
entity keys come back sorted, query results in primary key order, search
"distance" is the cosine similarity (higher is closer), and an upsert
replaces the whole entity (static fields it leaves out become null, other
keys go to the dynamic field). Collections:
  default/docs    VarChar pk "id", FloatVector "vector" (dim 3), nullable
                  Int64 "year", dynamic field; a, b, c
  prod/docs       the same schema; z only, to tell the databases apart
  default/custom  VarChar pk "doc_id", FloatVector "emb" (dim 3), dynamic
                  field; empty. Like Milvus, a search hit also carries the pk
                  under "id" when it has another name
  default/autoid  Int64 autoID pk (unsupported by Brix)
Filters understand `field op literal` (== != > < >= <=), `field in [...]`
and "" (everything); anything else is a parse error. Auth: no token, or
"root:Milvus". Unlike Milvus, a request of more than 1000 entities (upsert)
or ids (delete) is rejected, so the batching tests prove the splitting; id
"reject-me" fails its upsert request (all or nothing, like Milvus). State
lives in this process.
"""
import json
import math
import re
import sys
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer

MAX_BATCH = 1000
PREFIX = "/v2/vectordb/"


def schema(pk_type="VarChar", auto_id=False, pk="id", vec="vector", year=True):
    fields = [
        {"autoId": auto_id, "name": pk, "nullable": False, "primaryKey": True,
         "type": pk_type,
         "params": [{"key": "max_length", "value": "64"}] if pk_type == "VarChar" else []},
        {"autoId": False, "name": vec, "nullable": False, "primaryKey": False,
         "type": "FloatVector", "params": [{"key": "dim", "value": "3"}]},
    ]
    if pk_type == "VarChar" and year:
        fields.append({"autoId": False, "name": "year", "nullable": True,
                       "primaryKey": False, "type": "Int64"})
    return fields


COLLECTIONS = {
    ("default", "docs"): {"fields": schema(), "rows": {
        "a": {"id": "a", "vector": [1.0, 0.0, 0.0], "genre": "news", "year": 2024},
        "b": {"id": "b", "vector": [0.6, 0.8, 0.0], "genre": "blog", "year": None},
        "c": {"id": "c", "vector": [0.0, 1.0, 0.0], "year": None},
    }},
    ("prod", "docs"): {"fields": schema(), "rows": {
        "z": {"id": "z", "vector": [0.0, 0.0, 1.0], "year": None},
    }},
    ("default", "custom"): {"fields": schema(pk="doc_id", vec="emb", year=False),
                            "rows": {}},
    ("default", "autoid"): {"fields": schema("Int64", True), "rows": {}},
}


def pk_of(coll):
    return coll["fields"][0]["name"]


def vec_of(coll):
    return coll["fields"][1]["name"]


class Fail(Exception):
    def __init__(self, code, message):
        super().__init__(message)
        self.code, self.message = code, message


def cosine(u, v):
    nu = math.sqrt(sum(x * x for x in u))
    nv = math.sqrt(sum(x * x for x in v))
    if nu == 0 or nv == 0:
        return 0.0
    return sum(x * y for x, y in zip(u, v)) / (nu * nv)


LITERAL = r'"(?:[^"\\]|\\.)*"|-?[0-9]+(?:\.[0-9]+)?'


def parse_literal(text):
    return json.loads(text)


def parse_filter(expr):
    """A predicate over a row for the small expression subset above."""
    expr = expr.strip()
    if expr == "":
        return lambda row: True
    m = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)\s+in\s+\[(.*)\]", expr, re.S)
    if m:
        items = re.findall(LITERAL, m.group(2))
        if re.sub(LITERAL, "", m.group(2)).replace(",", "").strip():
            raise ValueError
        values = [parse_literal(x) for x in items]
        field = m.group(1)
        return lambda row: row.get(field) in values
    m = re.fullmatch(r"([A-Za-z_][A-Za-z0-9_]*)\s*(==|!=|>=|<=|>|<)\s*(" + LITERAL + ")", expr)
    if not m:
        raise ValueError
    field, op, want = m.group(1), m.group(2), parse_literal(m.group(3))

    def test(row):
        have = row.get(field)
        if have is None or type(have) is not type(want) and not (
                isinstance(have, (int, float)) and isinstance(want, (int, float))):
            return False
        return {"==": have == want, "!=": have != want, ">": have > want,
                "<": have < want, ">=": have >= want, "<=": have <= want}[op]
    return test


def predicate(body):
    try:
        return parse_filter(body.get("filter") or "")
    except ValueError:
        expr = body.get("filter")
        raise Fail(1100, f"failed to create query plan: cannot parse expression: {expr}, "
                         f"error: line 1:{len(expr)} mismatched input '<EOF>'")


def project(row, fields, out):
    if out is None or "*" in out:
        names = [f["name"] for f in fields] + [k for k in row if k not in {f["name"] for f in fields}]
    else:
        names = out
    return {k: row.get(k) for k in names}


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, status, body, content_type="application/json"):
        data = body if isinstance(body, bytes) else json.dumps(body, sort_keys=True, separators=(",", ":")).encode()
        self.send_response(status)
        self.send_header("Content-Type", content_type)
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def do_POST(self):
        raw = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if self.headers.get("Authorization") not in (None, "Bearer root:Milvus"):
            return self.reply(401, {"code": 1800, "message": "user hasn't authenticated"})
        route = self.path[len(PREFIX):] if self.path.startswith(PREFIX) else None
        handler = {
            "collections/describe": self.describe,
            "entities/query": self.query,
            "entities/search": self.search,
            "entities/get": self.get,
            "entities/upsert": self.upsert,
            "entities/delete": self.delete,
        }.get(route)
        if handler is None:
            return self.reply(404, b"404 page not found", "text/plain")
        if self.headers.get("Content-Type") != "application/json":
            return self.reply(200, {"code": 1801, "message": "can only accept json format request"})
        try:
            body = json.loads(raw)
            db = body.get("dbName", "default")
            name = body.get("collectionName")
            coll = COLLECTIONS.get((db, name))
            if coll is None:
                raise Fail(100, f"can't find collection[database={db}][collection={name}]")
            data = handler(coll, body)
        except Fail as e:
            return self.reply(200, {"code": e.code, "message": e.message})
        return self.reply(200, {"code": 0, "cost": 0, "data": data})

    def describe(self, coll, body):
        return {"autoId": coll["fields"][0]["autoId"], "collectionName": body["collectionName"],
                "consistencyLevel": "Strong", "enableDynamicField": True,
                "fields": coll["fields"], "load": "LoadStateLoaded"}

    def query(self, coll, body):
        test = predicate(body)
        rows = [r for _, r in sorted(coll["rows"].items()) if test(r)]
        out = body.get("outputFields")
        if out == ["count(*)"]:
            return [{"count(*)": len(rows)}]
        if not body.get("filter") and "limit" not in body:
            raise Fail(1100, "empty expression should be used with limit")
        rows = rows[:body.get("limit", len(rows))]
        return [project(r, coll["fields"], out) for r in rows]

    def search(self, coll, body):
        (vec,) = body["data"]
        if len(vec) != 3:
            raise Fail(1801, "can only accept json format request, error: dimension: 3, "
                             f"but length of []float: {len(vec)}: invalid parameter")
        test = predicate(body)
        hits = sorted(((round(cosine(vec, r[vec_of(coll)]), 6), k, r)
                       for k, r in coll["rows"].items() if test(r)),
                      key=lambda h: (-h[0], h[1]))[:body["limit"]]
        return [dict(project(r, coll["fields"], body.get("outputFields")), distance=d, id=k)
                for d, k, r in hits]

    def get(self, coll, body):
        out = body.get("outputFields")
        return [project(coll["rows"][i], coll["fields"], out)
                for i in body["id"] if i in coll["rows"]]

    def upsert(self, coll, body):
        rows = body["data"]
        if len(rows) > MAX_BATCH:
            raise Fail(1100, f"mock: more than {MAX_BATCH} entities per request")
        static = [f["name"] for f in coll["fields"]]
        pk, vec = pk_of(coll), vec_of(coll)
        for r in rows:
            if r.get(pk) == "reject-me":
                raise Fail(1100, "rejected")
            if len(r.get(vec, [])) != 3:
                raise Fail(1804, "fail to deal the insert data, error: []float32 size "
                                 f"{len(r.get(vec, []))} doesn't equal to vector "
                                 "dimension 3 of FloatVector")
        for r in rows:  # all or nothing: checked first
            coll["rows"][r[pk]] = dict({k: None for k in static}, **r)
        return {"upsertCount": len(rows), "upsertIds": [r[pk] for r in rows]}

    def delete(self, coll, body):
        test = predicate(body)
        doomed = [k for k, r in coll["rows"].items() if test(r)]
        if len(re.findall(LITERAL, body.get("filter", ""))) > MAX_BATCH:
            raise Fail(1100, f"mock: more than {MAX_BATCH} ids per request")
        for k in doomed:
            del coll["rows"][k]
        return {"deleteCount": len(doomed)}


def main():
    server = ThreadingHTTPServer(("127.0.0.1", 0), Handler)
    print(server.server_address[1], flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())
