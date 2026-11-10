#!/usr/bin/env python3
"""Interpretation regression for corrected records section 15.1 examples.

Spec 7.1: every possible record at a head test must satisfy its fields.
The coordinator corrected the broad-input fixture: it must be rejected.
This checks the closed K = "a" instance against the accepted map model.
It is not a frontend test: record syntax is not implemented yet.
"""

import importlib.util
from pathlib import Path
import sys

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "bdd_model", Path(__file__).with_name("t3-record-bdd.py")
)
model = importlib.util.module_from_spec(spec)
spec.loader.exec_module(model)

kernel = model.Kernel()
# Bits 1 and 2 stand for present Int representatives; bit 8 is absence.
# With K = "a", { ..{ a: Int }, K: Int } is just { a: Int }.
target = kernel.leaf({"a": 3})
assert kernel.inclusion(1, target) == "Refuted"
assert not model.members({"a": 3}, {(model.ABSENT,)})
assert kernel.inclusion(target, target) == "Proven"

# In the removal variant, K = "b" leaves the required a field too.
removed = kernel.project(kernel.leaf({"a": 3}), {"b"})
assert kernel.inclusion(1, removed) == "Refuted"
assert kernel.inclusion(target, removed) == "Proven"
print("Broad inputs refuted in both variants; field-constrained repairs proven.")
print("The empty record witnesses both required broad-input rejections.")
