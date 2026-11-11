#!/usr/bin/env python3
"""Export the accepted BDD model's record inclusion and projection verdicts.

Required by records.md sections 21.3 and 21.6. The frontend does not expose
negative record predicates, so projection comparisons target the checker core.
"""
import importlib.util
from itertools import combinations_with_replacement, product
from pathlib import Path
import sys

sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location(
    "bdd_model", Path(__file__).with_name("t3-record-bdd.py")
)
model = importlib.util.module_from_spec(spec)
spec.loader.exec_module(model)
k = model.Kernel()
universe = tuple(product((1, 2, 4, model.ABSENT), repeat=2))
fields = [
    {label: bits for label, bits in zip("ab", row) if bits}
    for row in product(range(8), repeat=2)
]
roots = [k.leaf(fs) for fs in fields]
for i, j in combinations_with_replacement(range(64), 2):
    k.work = 0
    target = k.op("or", roots[i], roots[j])
    mask = sum(
        int(k.inclusion(source, target) == "Proven") << n
        for n, source in enumerate(roots)
    )
    print("inclusion", i, j, mask)
for i in range(64):
    for j in range(64):
        k.work = 0
        root = k.op("diff", roots[i], roots[j])
        for index, labels in enumerate((set(), {"a"}, {"b"}, {"a", "b"})):
            k.work = 0
            projected = k.project(root, labels)
            mask = sum(
                int(k.truth(projected, [
                    all(row[ord(label) - 97] & bits for label, bits in fs)
                    for fs in k.atoms
                ])) << n
                for n, row in enumerate(universe)
            )
            print("projection", i, j, index, mask)
