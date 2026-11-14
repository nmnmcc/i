#!/usr/bin/env python3
"""Writes a package with a union of one-constructor types and an
unannotated recursive match over it (syntax three, stage 1, B2).

usage: scripts/gen-syn3-union.py <members> <directory>
"""
import os
import sys

n, out = int(sys.argv[1]), sys.argv[2]
os.makedirs(os.path.join(out, "src"), exist_ok=True)
with open(os.path.join(out, "i.toml"), "w") as f:
    f.write("# spec : 4.4, 7.4\n# expect : pass\n\nname = \"spec\"\n")
lines = ["C0 { C0 }"]
lines += [f"C{i}(A) {{ C{i}(A, E(A)) }}" for i in range(1, n)]
lines.append("E(A) = C0 | " + " | ".join(f"C{i}(A)" for i in range(1, n)))
lines.append("f0 = (e) {\n\te |> {\n\t\tC0 -> 0")
lines += [f"\t\tC{i}(a, r) -> a + f0(r)" for i in range(1, n)]
lines.append("\t}\n}")
lines.append("main = () { assert(f0(C1(1, C2(2, C0))) == 3) }")
with open(os.path.join(out, "src", "main.i"), "w") as f:
    f.write("\n".join(lines) + "\n")
