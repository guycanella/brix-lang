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
"""
import os

FLOAT = 1  # TensorProto.FLOAT


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


def value_info(name, dims):
    shape = b"".join(
        field(1, field(2, d) if isinstance(d, str) else field(1, d)) for d in dims
    )  # TensorShapeProto.dim: dim_param (2) or dim_value (1)
    tensor = field(1, FLOAT) + field(2, shape)  # TypeProto.Tensor
    return field(1, name) + field(2, field(1, tensor))  # ValueInfoProto


def node(op, inputs, outputs, name):
    return (b"".join(field(1, i) for i in inputs)
            + b"".join(field(2, o) for o in outputs)
            + field(3, name) + field(4, op))


def model():
    graph = (field(1, node("Add", ["x", "y"], ["sum"], "add"))
             + field(1, node("Mul", ["sum", "scale"], ["scaled"], "mul"))
             + field(2, "multi")
             + field(11, value_info("x", ["batch", 3]))
             + field(11, value_info("y", [1, 3]))
             + field(11, value_info("scale", []))
             + field(12, value_info("sum", ["batch", 3]))
             + field(12, value_info("scaled", ["batch", 3])))
    return (field(1, 8)                           # ir_version
            + field(2, "brix-test")              # producer_name
            + field(7, graph)
            + field(8, field(1, "") + field(2, 17)))  # opset_import


if __name__ == "__main__":
    here = os.path.dirname(os.path.abspath(__file__))
    with open(os.path.join(here, "multi.onnx"), "wb") as f:
        f.write(model())
    with open(os.path.join(here, "broken.onnx"), "wb") as f:
        f.write(b"this is not an ONNX model\n")
