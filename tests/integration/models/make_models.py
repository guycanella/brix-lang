#!/usr/bin/env python3
"""Writes the small ONNX models the onnx tests load (stdlib only: the
protobuf is encoded by hand). Run from the repo root:

    python3 tests/integration/models/make_models.py

multi.onnx (opset 17), 3 inputs and 2 outputs:
    x     float [batch, 3]   (a dynamic dimension)
    y     float [1, 3]
    scale float []           (a scalar)
    sum    = Add(x, y)       float [batch, 3]
    scaled = Mul(sum, scale) float [batch, 3]

typed.onnx (opset 17), one input per element type Brix converts:
    a int64  [n, 2, 3]       (rank 3, a dynamic dimension)
    w double []
    relu  = Relu(Cast(a, float))       float  [n, 2, 3]
    wd    = Mul(Cast(relu, double), w) double [n, 2, 3]
    trunc = Cast(wd, int32)            int32  [n, 2, 3]
"""
import os

FLOAT, INT32, INT64, DOUBLE = 1, 6, 7, 11  # TensorProto element types


def varint(n):
    out = bytearray()
    while True:
        b = n & 0x7F
        n >>= 7
        out.append(b | (0x80 if n else 0))
        if not n:
            return bytes(out)


def field(num, value):
    """One protobuf field: int -> varint, str/bytes -> length-delimited."""
    if isinstance(value, int):
        return varint(num << 3) + varint(value)
    data = value.encode() if isinstance(value, str) else value
    return varint(num << 3 | 2) + varint(len(data)) + data


def value_info(name, dims, elem=FLOAT):
    shape = b"".join(
        field(1, field(2, d) if isinstance(d, str) else field(1, d)) for d in dims
    )  # TensorShapeProto.dim: dim_param (2) or dim_value (1)
    tensor = field(1, elem) + field(2, shape)  # TypeProto.Tensor
    return field(1, name) + field(2, field(1, tensor))  # ValueInfoProto


def node(op, inputs, outputs, name, to=None):
    attr = b""
    if to is not None:  # Cast's `to`: AttributeProto name (1), i (3), type INT (20)
        attr = field(5, field(1, "to") + field(3, to) + field(20, 2))
    return (b"".join(field(1, i) for i in inputs)
            + b"".join(field(2, o) for o in outputs)
            + field(3, name) + field(4, op) + attr)


def wrap(graph):
    return (field(1, 8)                           # ir_version
            + field(2, "brix-test")              # producer_name
            + field(7, graph)
            + field(8, field(1, "") + field(2, 17)))  # opset_import


def typed():
    dims = ["n", 2, 3]
    return wrap(field(1, node("Cast", ["a"], ["af"], "cast_a", to=FLOAT))
                + field(1, node("Relu", ["af"], ["relu"], "relu"))
                + field(1, node("Cast", ["relu"], ["rd"], "cast_r", to=DOUBLE))
                + field(1, node("Mul", ["rd", "w"], ["wd"], "mul"))
                + field(1, node("Cast", ["wd"], ["trunc"], "cast_t", to=INT32))
                + field(2, "typed")
                + field(11, value_info("a", dims, INT64))
                + field(11, value_info("w", [], DOUBLE))
                + field(12, value_info("relu", dims))
                + field(12, value_info("wd", dims, DOUBLE))
                + field(12, value_info("trunc", dims, INT32)))


def model():
    graph = (field(1, node("Add", ["x", "y"], ["sum"], "add"))
             + field(1, node("Mul", ["sum", "scale"], ["scaled"], "mul"))
             + field(2, "multi")
             + field(11, value_info("x", ["batch", 3]))
             + field(11, value_info("y", [1, 3]))
             + field(11, value_info("scale", []))
             + field(12, value_info("sum", ["batch", 3]))
             + field(12, value_info("scaled", ["batch", 3])))
    return wrap(graph)


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "multi.onnx"), "wb") as f:
        f.write(model())
    with open(os.path.join(here, "typed.onnx"), "wb") as f:
        f.write(typed())
    with open(os.path.join(here, "broken.onnx"), "wb") as f:
        f.write(b"this is not an ONNX model\n")
