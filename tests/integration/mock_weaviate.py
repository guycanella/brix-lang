#!/usr/bin/env python3
"""Minimal mock of a Weaviate server (REST /v1 + GraphQL) for the vectordb tests.

Listens on 127.0.0.1 on a free port and prints the port on the first line of
stdout. Responses follow a real Weaviate 1.40 server: the same shapes and
status codes, a batch that answers 200 with an error per failed object,
GraphQL errors as 200 + "errors", objects replaced whole on upsert,
auto-schema (a write to a missing class creates it, as the real server does
-- the runtime must never let that happen), a 404 with an empty body for a
missing object or class, and list pages ordered by UUID with the `after`
cursor. The GraphQL text is really parsed (selections, arguments, input
values), so a malformed query from the runtime fails.
Class Docs (dimension 3, cosine): a, b, c (written by Brix, UUIDv5 ids with
a brix_id property) and one object without brix_id; its schema also has
object, geoCoordinates, phoneNumber and cross-reference properties, which a
query must not select without a sub-selection. Class names are looked up the
way Weaviate does (first letter capitalized: "docs" is Docs). Class Vanish
disappears once its schema has been read (a class deleted after the runtime
cached it).
Auth: no token is accepted (anonymous access); a token must be
"Bearer test-key", else 401. Test hooks: a batch of more than 100 objects is
rejected (proves the splitting), and brix_id "reject-me" fails its object.
"""
import json
import math
import re
import sys
import uuid
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from urllib.parse import parse_qs, urlsplit

DIMENSION = 3
MAX_BATCH = 100


def uid(brix_id):
    return str(uuid.uuid5(uuid.NAMESPACE_DNS, brix_id))


LEGACY = "11111111-2222-4333-8444-555555555555"

# class -> {"props": {name: dataType}, "objects": {uuid: (vector, properties)}}
CLASSES = {
    "Docs": {
        "props": {"brix_id": "text", "genre": "text", "year": "int", "loc": "object",
                  "place": "geoCoordinates", "phone": "phoneNumber", "cites": "Article",
                  "tags": "text[]"},
        "objects": {
            uid("a"): ([1.0, 0.0, 0.0], {"brix_id": "a", "genre": "news", "year": 2024}),
            uid("b"): ([0.6, 0.8, 0.0], {"brix_id": "b", "genre": "blog", "loc": {"x": 1}}),
            uid("c"): ([0.0, 1.0, 0.0], {"brix_id": "c"}),
            LEGACY: ([0.0, 0.0, 1.0], {"genre": "news"}),
        },
    },
    "Vanish": {"props": {"brix_id": "text"}, "objects": {}},
}
SCALARS = {"text", "int", "number", "boolean", "date", "uuid", "blob"}


def canonical(name):
    return name[:1].upper() + name[1:]


def data_type(v):
    if isinstance(v, bool):
        return "boolean"
    if isinstance(v, int):
        return "int"
    if isinstance(v, float):
        return "number"
    if isinstance(v, dict):
        return "object"
    return "text"


def cosine_distance(u, v):
    nu = math.sqrt(sum(x * x for x in u))
    nv = math.sqrt(sum(x * x for x in v))
    if nu == 0 or nv == 0:
        return 1.0
    return 1.0 - sum(x * y for x, y in zip(u, v)) / (nu * nv)


# ---- GraphQL: just enough of the grammar for what the runtime sends.

TOKEN = re.compile(r'\s+|,|("(?:\\.|[^"\\])*")|(-?\d+(?:\.\d+)?(?:[eE][-+]?\d+)?)|([_A-Za-z]\w*)|([{}()\[\]:])')


class Enum(str):
    pass


def tokenize(text):
    pos, out = 0, []
    while pos < len(text):
        m = TOKEN.match(text, pos)
        if not m:
            raise ValueError(f"Syntax Error: unexpected character at {pos}")
        pos = m.end()
        if m.group(1):
            out.append(("str", json.loads(m.group(1))))
        elif m.group(2):
            n = m.group(2)
            out.append(("num", float(n) if any(c in n for c in ".eE") else int(n)))
        elif m.group(3):
            out.append(("name", m.group(3)))
        elif m.group(4):
            out.append(("punct", m.group(4)))
    return out


class Parser:
    def __init__(self, text):
        self.toks = tokenize(text)
        self.i = 0

    def peek(self):
        return self.toks[self.i] if self.i < len(self.toks) else ("eof", None)

    def take(self, kind=None, value=None):
        tok = self.peek()
        if (kind and tok[0] != kind) or (value is not None and tok[1] != value):
            raise ValueError(f"Syntax Error: expected {value or kind}, got {tok[1]!r}")
        self.i += 1
        return tok[1]

    def selection_set(self):
        self.take("punct", "{")
        fields = []
        while self.peek() != ("punct", "}"):
            name = self.take("name")
            args = {}
            if self.peek() == ("punct", "("):
                self.take()
                while self.peek() != ("punct", ")"):
                    key = self.take("name")
                    self.take("punct", ":")
                    args[key] = self.value()
                self.take()
            sub = self.selection_set() if self.peek() == ("punct", "{") else None
            fields.append((name, args, sub))
        self.take()
        return fields

    def value(self):
        kind, v = self.peek()
        if (kind, v) == ("punct", "{"):
            self.take()
            obj = {}
            while self.peek() != ("punct", "}"):
                key = self.take("name")
                self.take("punct", ":")
                obj[key] = self.value()
            self.take()
            return obj
        if (kind, v) == ("punct", "["):
            self.take()
            items = []
            while self.peek() != ("punct", "]"):
                items.append(self.value())
            self.take()
            return items
        self.take()
        if kind == "name":
            return {"true": True, "false": False, "null": None}.get(v, Enum(v))
        if kind in ("str", "num"):
            return v
        raise ValueError(f"Syntax Error: unexpected {v!r}")


def matches_where(props, where):
    op = where["operator"]
    if not isinstance(op, Enum):
        raise ValueError("operator must be an enum")
    if op in ("And", "Or"):
        results = [matches_where(props, w) for w in where["operands"]]
        return all(results) if op == "And" else any(results)
    (prop,) = where["path"]
    value = next(v for k, v in where.items() if k.startswith("value"))
    actual = props.get(prop)
    return {
        "Equal": actual == value,
        "NotEqual": actual != value,
        "GreaterThan": actual is not None and actual > value,
        "LessThan": actual is not None and actual < value,
    }[op]


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):
        pass

    def reply(self, status, body=None):
        data = b"" if body is None else json.dumps(body).encode()
        self.send_response(status)
        if data:
            self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(data)))
        self.end_headers()
        self.wfile.write(data)

    def authorized(self):
        auth = self.headers.get("Authorization")
        if auth is not None and auth != "Bearer test-key":
            self.reply(401, {"code": 401, "message": "anonymous access not enabled, "
                                                     "please provide an auth scheme"})
            return False
        return True

    def body(self):
        raw = self.rfile.read(int(self.headers.get("Content-Length", 0)))
        if self.headers.get("Content-Type") != "application/json":
            self.reply(415, {"code": 415, "message": "Content-Type must be application/json"})
            return None
        return json.loads(raw)

    def do_GET(self):
        if not self.authorized():
            return
        if self.headers.get("Content-Type"):
            return self.reply(400, {"error": [{"message": "GET must not send a Content-Type"}]})
        url = urlsplit(self.path)
        parts = url.path.split("/")
        params = parse_qs(url.query)
        if len(parts) == 4 and parts[2] == "schema":
            name = canonical(parts[3])
            cls = CLASSES.get(name)
            if cls is None:
                return self.reply(404)
            if name == "Vanish":  # test hook: deleted right after this read
                del CLASSES[name]
            return self.reply(200, {
                "class": name, "vectorizer": "none",
                "properties": [{"name": n, "dataType": [t]} for n, t in cls["props"].items()],
            })
        if len(parts) == 5 and parts[2] == "objects":
            cls = CLASSES.get(parts[3])
            obj = cls and cls["objects"].get(parts[4])
            if obj is None:
                return self.reply(404)
            out = {"class": parts[3], "id": parts[4], "properties": obj[1]}
            if params.get("include") == ["vector"]:
                out["vector"] = obj[0]
            return self.reply(200, out)
        if url.path == "/v1/objects":
            cls = CLASSES.get(params.get("class", [""])[0])
            if cls is None:
                return self.reply(404)
            ids = sorted(cls["objects"])
            after = params.get("after", [None])[0]
            if after is not None:
                ids = [i for i in ids if i > after]
            limit = int(params.get("limit", ["25"])[0])
            objs = [{"class": params["class"][0], "id": i, "properties": cls["objects"][i][1]}
                    for i in ids[:limit]]
            return self.reply(200, {"objects": objs, "totalResults": len(objs)})
        return self.reply(404)

    def do_POST(self):
        if not self.authorized():
            return
        body = self.body()
        if body is None:
            return
        if self.path == "/v1/graphql":
            return self.graphql(body["query"])
        if self.path == "/v1/batch/objects":
            return self.batch_upsert(body["objects"])
        return self.reply(404)

    def do_DELETE(self):
        if not self.authorized():
            return
        body = self.body()
        if body is None:
            return
        if self.path != "/v1/batch/objects":
            return self.reply(404)
        match = body["match"]
        where = match["where"]
        ids = where["valueTextArray"]
        if where["path"] != ["id"] or where["operator"] != "ContainsAny":
            return self.reply(422, {"error": [{"message": "unexpected where"}]})
        if len(ids) > MAX_BATCH:
            return self.reply(422, {"error": [{"message": f"more than {MAX_BATCH} ids"}]})
        objects = CLASSES.get(match["class"], {"objects": {}})["objects"]
        found = [i for i in ids if i in objects]
        for i in found:
            del objects[i]
        return self.reply(200, {"match": match, "output": "minimal", "dryRun": False,
                                "results": {"matches": len(found), "successful": len(found),
                                            "failed": 0, "limit": 10000, "objects": None}})

    def batch_upsert(self, objects):
        if len(objects) > MAX_BATCH:
            return self.reply(422, {"error": [{"message": f"more than {MAX_BATCH} objects"}]})
        out = []
        for obj in objects:
            cls = CLASSES.setdefault(obj["class"], {"props": {}, "objects": {}})  # auto-schema
            vec, props = obj["vector"], obj.get("properties", {})
            result = {"status": "SUCCESS"}
            if props.get("brix_id") == "reject-me":
                result = {"status": "FAILED", "errors": {"error": [{"message": "rejected"}]}}
            elif cls["objects"] and len(vec) != DIMENSION:
                result = {"status": "FAILED", "errors": {"error": [{"message":
                          f"Validate vector index for {obj['id']}: new node has a vector "
                          f"with length {len(vec)}. Existing nodes have vectors with length "
                          f"{DIMENSION}: vector dimensions do not match the index dimensions"}]}}
            else:
                for k, v in props.items():
                    cls["props"].setdefault(k, data_type(v))
                cls["objects"][obj["id"]] = (vec, props)
            out.append(dict(obj, result=result))
        return self.reply(200, out)

    def graphql(self, text):
        try:
            ((root, _, fields),) = Parser(text).selection_set()
            ((name, args, sub),) = fields
            cls = CLASSES.get(name)
            if cls is None:
                raise ValueError(f'Cannot query field "{name}" on type "{root}".')
            if root == "Aggregate":
                return self.reply(200, {"data": {"Aggregate": {name: [{"meta": {"count": len(cls["objects"])}}]}}})
            return self.reply(200, {"data": {"Get": {name: self.get(cls, args, sub)}}})
        except (ValueError, KeyError) as err:
            msg = err.args[0] if err.args else "query failed"
            return self.reply(200, {"data": None, "errors": [{"message": str(msg), "path": None}]})

    def get(self, cls, args, sub):
        vec = args["nearVector"]["vector"]
        if len(vec) != DIMENSION:
            raise ValueError(f"vector search: knn search: vector lengths don't match: "
                             f"{DIMENSION} vs {len(vec)}")
        where = args.get("where")
        if where is not None:
            for p in re.findall(r"'path': \['(\w+)'\]", repr(where)):
                if p not in cls["props"]:
                    raise ValueError(f"invalid 'where' filter: no such prop with name '{p}' "
                                     f"found in class in the schema")
        hits = sorted(
            (cosine_distance(vec, v), oid, props)
            for oid, (v, props) in cls["objects"].items()
            if where is None or matches_where(props, where)
        )[:args["limit"]]
        out = []
        for dist, oid, props in hits:
            row = {}
            for field, _, fsub in sub:
                if field == "_additional":
                    row[field] = {"id": oid, "distance": round(dist, 6)}
                elif field not in cls["props"]:
                    raise ValueError(f'Cannot query field "{field}"')
                elif cls["props"][field].rstrip("[]") not in SCALARS and fsub is None:
                    raise ValueError(f'Field "{field}" of type "{cls["props"][field]}" '
                                     f'must have a selection of subfields.')
                else:
                    row[field] = props.get(field)
            out.append(row)
        return out


class Server(ThreadingHTTPServer):
    def handle_error(self, request, client_address):
        pass


def main():
    server = Server(("127.0.0.1", 0), Handler)
    print(server.server_address[1], flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass


if __name__ == "__main__":
    sys.exit(main())
