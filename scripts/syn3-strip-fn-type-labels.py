#!/usr/bin/env python3
"""Syntax three, stage 4: labels of function type parameters in the code
spans of Markdown table rows (`(key: String) -> R` becomes `(String) -> R`).
Usage: scripts/syn3-strip-fn-type-labels.py FILE..."""
import re
import sys

LABEL = re.compile(r"[a-z_][a-z0-9_]*: ")


def strip(code):
    out = list(code)
    stack = []
    groups = []
    for i, c in enumerate(code):
        if c in "([{":
            stack.append(i)
        elif c in ")]}" and stack:
            o = stack.pop()
            if c == ")" and code[i + 1 :].startswith(" ->"):
                groups.append((o, i))
    cut = set()
    for o, e in groups:
        depth = 0
        k = o + 1
        start = True
        while k < e:
            c = code[k]
            if start:
                m = LABEL.match(code, k)
                if m:
                    cut.update(range(k, m.end()))
                start = False
            if c in "([{":
                depth += 1
            elif c in ")]}":
                depth -= 1
            elif c == "," and depth == 0:
                start = True
                k += 1
                while k < e and code[k] == " ":
                    k += 1
                continue
            k += 1
    return "".join(ch for i, ch in enumerate(out) if i not in cut)


for path in sys.argv[1:]:
    text = open(path).read()
    lines = text.split("\n")
    for n, line in enumerate(lines):
        if line.startswith("|"):
            lines[n] = re.sub(r"`([^`]*->[^`]*)`", lambda m: "`" + strip(m.group(1)) + "`", line)
    new = "\n".join(lines)
    if new != text:
        open(path, "w").write(new)
