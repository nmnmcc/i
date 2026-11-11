#!/usr/bin/env python3
"""Accepted eager and compact models supply finite source-graph denotations.

The protocol is a topological source graph, not a serialized expected Rust
representation. Verdicts come from the eager model's independent set oracle.
The compact model must agree too. Production frontend checks follow later.
"""
import importlib.util
from itertools import combinations_with_replacement
from pathlib import Path
import sys

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "compact", Path(__file__).with_name("t3-record-demand.py")
)
model = importlib.util.module_from_spec(spec)
spec.loader.exec_module(model)
e = model.e
compact = model.Graph()
ids = {}


def emit(tree):
    if tree in ids:
        return ids[tree]
    kind = tree[0]
    if kind == "rec":
        fields = [(ord(k) - 97, emit(v)) for k, v in tree[1]]
        words = ["rec", len(fields)] + [item for field in fields for item in field]
    elif kind in ("union", "inter"):
        words = [kind, *(emit(child) for child in tree[1])]
    elif kind == "without":
        words = [kind, emit(tree[1]), sum(1 << (ord(k) - 97) for k in tree[2])]
    else:
        words = [{"never": "never", "top": "top", "int": "int", "lit": "one", "var": "var"}[kind]]
    index = len(ids)
    ids[tree] = index
    print("node", *words)
    return index


def check(tree, replacement=None):
    source = emit(tree)
    c = compact.import_tree(tree)
    if replacement is not None:
        key = ("substitute", tree, replacement)
        if key not in ids:
            value = emit(replacement)
            ids[key] = len(ids)
            print("node", "subst", source, value)
        source = ids[key]
        tree = e.subst(tree, replacement)
        c = compact.subst(c, {"A": compact.import_tree(replacement)})
    expected = e.den(tree)
    assert e.den(compact.export_tree(c)) == expected
    print("check", source, expected[0], expected[1])


for left, right in combinations_with_replacement(e.CLOSED, 2):
    for operation in ("union", "inter"):
        for labels in e.MASKS:
            check(e.without((operation, (left, right)), labels))
for source in e.OPEN + [e.A, e.N]:
    for labels in e.MASKS:
        for replacement in (e.N, e.ONE, e.I):
            check(e.without(source, labels), replacement)
