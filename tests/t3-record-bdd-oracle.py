#!/usr/bin/env python3
"""Stream the accepted BDD model's small Boolean verdicts to the Rust test.

This is a core-model differential test, not yet a frontend compiler test.
The denotation is the eight independent assignments used by check_boolean.
"""

import importlib.util
from itertools import product
from pathlib import Path
import sys

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "bdd_model", Path(__file__).with_name("t3-record-bdd.py")
)
model = importlib.util.module_from_spec(spec)
spec.loader.exec_module(model)
kernel = model.Kernel()
assignments = tuple(product((False, True), repeat=3))
forms = [0, 1] + [kernel.leaf({chr(97 + n): 1}) for n in range(3)]


def emit(kind, op, left, right):
    kernel.work = 0
    root = kernel.op(op, forms[left], forms[right])
    mask = sum(int(kernel.truth(root, values)) << n for n, values in enumerate(assignments))
    print(kind, op, left, right, mask)
    return root


for operation in ("or", "and", "diff"):
    for left in range(5):
        for right in range(5):
            forms.append(emit("build", operation, left, right))
for left in range(len(forms)):
    for right in range(len(forms)):
        for operation in ("or", "and", "diff"):
            emit("check", operation, left, right)
