#!/usr/bin/env python3
"""Count executed allocations/entries in printed, final O2 WAT; never time it.

Usage: r2-profile.py input.wat output.wat counters.tsv
Run r2_performance::count_instrumented_kernels on the assembled output.
"""
import pathlib
import re
import sys

source, output, index = map(pathlib.Path, sys.argv[1:4])
function_names = {}
type_names = {}
if len(sys.argv) == 5:
    named = pathlib.Path(sys.argv[4]).read_text()
    function_names = {int(n): name for name, n in re.findall(r"^  \(func (\$\S+) \(;([0-9]+);\)", named, re.M)}
    type_names = {int(n): name for name, n in re.findall(r"\(type (\$\S+) \(;([0-9]+);\)", named)}
lines = source.read_text().splitlines()
rows = []
globals_ = []
result = []
function = None
pending_entry = False


def counter(kind, detail):
    name = f"r2.count.{len(rows)}"
    symbol = f"$r2_count_{len(rows)}"
    number = int(re.search(r"\(;([0-9]+);\)", function)[1])
    owner = function_names.get(number, function)
    if kind == "allocation":
        op, type_id = detail.split()
        if type_id.isdigit():
            detail = op + " " + type_names.get(int(type_id), type_id)
    rows.append((name, owner, kind, detail))
    globals_.append(f'  (global {symbol} (export "{name}") (mut i64) (i64.const 0))')
    return [f"    global.get {symbol}", "    i64.const 1", "    i64.add", f"    global.set {symbol}"]


for line in lines:
    match = re.match(r"  \(func (.*?)\(type ", line)
    if match:
        function = match[1].strip()
        pending_entry = True
        if line.count("(") == line.count(")"):
            result.append(line[:-1])
            result.extend(counter("entry", "function"))
            result.append("  )")
            function = None
            pending_entry = False
            continue
    if function and pending_entry and line.startswith("    ") and not line.lstrip().startswith("(local "):
        result.extend(counter("entry", "function"))
        pending_entry = False
    allocation = re.match(r"\s+((?:struct|array)\.new\S*)\s+(\S+)", line)
    if function and allocation:
        result.extend(counter("allocation", " ".join(allocation.groups())))
    result.append(line)
    if line == "  )":
        if pending_entry:
            result[-1:-1] = counter("entry", "function")
        function = None
        pending_entry = False

assert result[-1] == ")"
result[-1:] = globals_ + [")"]
output.write_text("\n".join(result) + "\n")
index.write_text("\n".join("\t".join(row) for row in rows) + "\n")
print(f"{len(rows)} counters; instrumentation only, not a timing artifact")
