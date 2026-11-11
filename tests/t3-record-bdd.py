#!/usr/bin/env python3
"""Design model: Elixir-style lazy BDD + open-map emptiness/projection.

Read first: .rd/ref/elixir/lib/elixir/lib/module/types/descr.ex:
bdd_union/2, bdd_intersection/2, bdd_difference/2, map_bdd_empty?/4,
map_line_meet_empty?/6, map_split_key_line/5, map_remove_negative/3.
This is a small independent transcription of their set equations, not a port
of the full checker. No dynamic, recursive, function or nominal leaf theory.
The three present representatives are 1, 2, "x"; bit 8 is field absence.
Unknown and ResourceLimit are not Boolean answers. See records.md section 21.
"""
import importlib.util
import sys
from itertools import combinations_with_replacement, product
from pathlib import Path
from time import perf_counter
sys.dont_write_bytecode = True

FREE = 15
ABSENT = 8


class ResourceLimit(Exception):
    pass


class Unknown(Exception):
    pass


class Kernel:
    def __init__(self, cache=True):
        self.nodes = [None, None]  # bottom, all records
        self.unique = {}
        self.atoms = []
        self.atom_ids = {}
        self.cache = {} if cache else None
        self.limit = 65536
        self.work = 0

    def tick(self):
        self.work += 1
        if self.work > self.limit:
            raise ResourceLimit("type too large to check here: annotation; annotate an intermediate record")

    def node(self, atom, yes, either, no):
        # Node = (atom & yes) | either | (~atom & no).
        if either == 1: return 1
        if yes == no: return self.op("or", yes, either)
        key = (atom, yes, either, no)
        if key not in self.unique:
            self.unique[key] = len(self.nodes)
            self.nodes.append(key)
        return self.unique[key]

    def leaf(self, fields):
        fs = tuple(sorted((k, v) for k, v in fields.items() if v != FREE))
        if any(v == 0 for _, v in fs): return 0
        if not fs: return 1
        if fs not in self.atom_ids:
            self.atom_ids[fs] = len(self.atoms)
            self.atoms.append(fs)
        return self.node(self.atom_ids[fs], 1, 0, 0)

    def op(self, op, a, b):
        if op in ("or", "and") and a > b: a, b = b, a
        key = (op, a, b)
        if self.cache is not None and key in self.cache: return self.cache[key]
        self.tick()
        if op == "or":
            if a == b or b == 0: out = a
            elif a == 0: out = b
            elif a == 1 or b == 1: out = 1
            else: out = self.apply(op, a, b)
        elif op == "and":
            if a == b or b == 1: out = a
            elif a == 1: out = b
            elif a == 0 or b == 0: out = 0
            else: out = self.apply(op, a, b)
        elif op == "diff":
            if a == 0 or a == b or b == 1: out = 0
            elif b == 0: out = a
            elif a == 1:
                h, c, u, d = self.nodes[b]
                out = self.op("and", self.node(h, self.op("diff", 1, c), 0,
                                              self.op("diff", 1, d)), self.op("diff", 1, u))
            else: out = self.apply(op, a, b)
        else: raise ValueError(op)
        if self.cache is not None: self.cache[key] = out  # completed results only
        return out

    def apply(self, op, a, b):
        h, c, u, d = self.nodes[a]
        j, e, v, f = self.nodes[b]
        if op == "or":
            if h < j: return self.node(h, c, self.op(op, u, b), d)
            if j < h: return self.node(j, e, self.op(op, v, a), f)
            return self.node(h, self.op(op, c, e), self.op(op, u, v), self.op(op, d, f))
        if op == "and":
            if h < j: return self.node(h, self.op(op, c, b), self.op(op, u, b), self.op(op, d, b))
            if j < h: return self.node(j, self.op(op, a, e), self.op(op, a, v), self.op(op, a, f))
            if u == v: return self.node(h, self.op(op, c, e), u, self.op(op, d, f))
            return self.node(h, self.op(op, self.op("or", c, u), self.op("or", e, v)), 0,
                             self.op(op, self.op("or", d, u), self.op("or", f, v)))
        if h < j: return self.node(h, self.op(op, c, b), self.op(op, u, b), self.op(op, d, b))
        if j < h:
            rest = self.op(op, a, v)
            return self.node(j, self.op(op, rest, e), 0, self.op(op, rest, f))
        return self.node(h, self.op(op, self.op("or", c, u), self.op("or", e, v)), 0,
                         self.op(op, self.op("or", d, u), self.op("or", f, v)))

    def truth(self, t, assignment):
        if t < 2: return bool(t)
        h, c, u, d = self.nodes[t]
        return self.truth(u, assignment) or self.truth(c if assignment[h] else d, assignment)

    def replace(self, root, atom_replacements):
        # Reinsert through Boolean operations: replacement can change atom order.
        done = {}
        def go(t):
            if t < 2: return t
            if t in done: return done[t]
            h, c, u, d = self.nodes[t]
            lit = atom_replacements.get(h, self.node(h, 1, 0, 0))
            out = self.op("or", self.op("and", lit, go(c)),
                          self.op("or", go(u), self.op("and", self.op("diff", 1, lit), go(d))))
            done[t] = out
            return out
        return go(root)

    def cells(self, root):
        # Stream positive cubes. Never construct the full DNF list.
        def subtract(pos, negs):
            self.tick()
            if not negs:
                yield pos
                return
            neg, *tail = negs
            prefix = dict(pos)
            for label, bits in neg:
                original = prefix.get(label, FREE)
                diff, meet = original & (FREE ^ bits), original & bits
                if diff:
                    residual = dict(prefix)
                    residual[label] = diff
                    yield from subtract(residual, tail)
                if not meet: return
                prefix[label] = meet
            # The residual after meeting every field is inside this negative.
        def walk(t, pos, negs):
            self.tick()
            if t == 0: return
            if t == 1:
                yield from subtract(pos, negs)
                return
            h, c, u, d = self.nodes[t]
            fs = self.atoms[h]
            if any(bits is None for _, bits in fs): raise Unknown("unresolved field")
            # Elixir's D/C/U traversal, with early positive-field emptiness.
            yield from walk(d, pos, (fs,) + negs)
            merged = dict(pos)
            for label, bits in fs:
                merged[label] = merged.get(label, FREE) & bits
                if not merged[label]: break
            else: yield from walk(c, merged, negs)
            yield from walk(u, pos, negs)
        yield from walk(root, {}, ())

    def empty(self, root):
        return next(self.cells(root), None) is None

    def project(self, root, labels):
        # Equivalent to split(field, rest), remove all negatives, then keep rest
        # only for inhabited field parts. Dropped fields become free, not absent.
        out = 0
        for cell in self.cells(root):
            out = self.op("or", out, self.leaf({k: v for k, v in cell.items() if k not in labels}))
        return out

    def inclusion(self, source, target, budget=65536):
        self.work, self.limit = 0, budget
        try: return "Proven" if self.empty(self.op("diff", source, target)) else "Refuted"
        except ResourceLimit: return "ResourceLimit"
        except Unknown: return "Unknown"

    def reachable(self, root):
        seen = set()
        def go(t):
            if t < 2 or t in seen: return
            seen.add(t)
            for child in self.nodes[t][1:]: go(child)
        go(root)
        return len(seen)


def members(fields, universe):
    return {r for r in universe if all(r[ord(k) - ord("a")] & v for k, v in fields.items())}


def check_boolean():
    counts = 0
    for cache in (True, False):
        k = Kernel(cache)
        atoms = [k.leaf({chr(97 + i): 1}) for i in range(3)]
        base = [0, 1] + atoms
        forms = [(t, tuple(k.truth(t, a) for a in product((False, True), repeat=3))) for t in base]
        for op in ("or", "and", "diff"):
            for a in base:
                for b in base:
                    k.work = 0
                    t = k.op(op, a, b)
                    forms.append((t, tuple(k.truth(t, v) for v in product((False, True), repeat=3))))
        for a, truth_a in forms:
            for b, truth_b in forms:
                for op in ("or", "and", "diff"):
                    k.work = 0
                    t = k.op(op, a, b)
                    expected = tuple((x or y) if op == "or" else (x and y) if op == "and" else (x and not y)
                                     for x, y in zip(truth_a, truth_b))
                    assert tuple(k.truth(t, v) for v in product((False, True), repeat=3)) == expected
                    counts += 1
        # Reverse atom order under substitution; compare by all truth assignments.
        old = k.op("diff", k.op("or", atoms[0], atoms[1]), atoms[2])
        replacement = {0: atoms[2], 1: atoms[0], 2: atoms[1]}
        new = k.replace(old, replacement)
        for v in product((False, True), repeat=3):
            assert k.truth(new, v) == ((v[2] or v[0]) and not v[1])
    # A symbolic record predicate can collapse to empty or become a compound
    # predicate after substitution. Rebuild, then compare with fresh construction.
    substitutions = 0
    k = Kernel()
    variable = k.leaf({"a": None})
    fixed = k.leaf({"b": 3})
    tail = k.leaf({"c": 1})
    expressions = [(op, k.op(op, variable, fixed)) for op in ("or", "and", "diff")]
    replacements = [k.leaf({"a": bits}) for bits in (0, 1, 3, 7)]
    replacements.append(k.op("or", k.leaf({"a": 1}), tail))
    atom = k.nodes[variable][0]
    for replacement in replacements:
        for op, root in expressions:
            k.work = 0
            rewritten = k.replace(root, {atom: replacement})
            rebuilt = k.op(op, replacement, fixed)
            assert k.inclusion(rewritten, rebuilt) == "Proven"
            assert k.inclusion(rebuilt, rewritten) == "Proven"
            substitutions += 1
    assert k.leaf({"a": 0}) == 0
    print("Boolean operation pairs (cache on/off):", counts, "; atom reorder: 16 assignments; substitutions:", substitutions)


def check_records():
    k = Kernel()
    universe = tuple(product((1, 2, 4, ABSENT), repeat=2))
    fields = [{label: bits for label, bits in zip("ab", row) if bits}
              for row in product(range(8), repeat=2)]
    roots = [k.leaf(fs) for fs in fields]
    denotations = [members(fs, universe) for fs in fields]
    count = 0
    for i, j in combinations_with_replacement(range(64), 2):
        k.work = 0
        target = k.op("or", roots[i], roots[j])
        expected = denotations[i] | denotations[j]
        for source, values in zip(roots, denotations):
            verdict = k.inclusion(source, target)
            assert verdict == ("Proven" if values <= expected else "Refuted"), (i, j, verdict)
            count += 1
    projections = 0
    # Negative maps and absent fields are essential: a positive-only DNF test
    # would miss the common-witness failure under projection.
    for i in range(64):
        for j in range(64):
            k.work = 0
            root = k.op("diff", roots[i], roots[j])
            values = denotations[i] - denotations[j]
            for labels in (set(), {"a"}, {"b"}, {"a", "b"}):
                k.work = 0
                projected = k.project(root, labels)
                expected = {r for r in universe if any(all(r[n] == s[n] for n, label in enumerate("ab")
                                                         if label not in labels) for s in values)}
                actual = {r for r in universe if k.truth(projected, [all(r[ord(label)-97] & bits for label, bits in fs)
                                                                    for fs in k.atoms])}
                assert actual == expected
                projections += 1
    # Common-witness attack: projection may not distribute over intersection.
    k.work = 0
    one, two = k.leaf({"a": 1}), k.leaf({"a": 2})
    assert k.project(k.op("and", one, two), {"a"}) == 0
    assert k.op("and", k.project(one, {"a"}), k.project(two, {"a"})) == 1
    # No resource failure is a semantic cache entry. Retry the same demand.
    assert k.inclusion(1, one, budget=1) == "ResourceLimit"
    assert k.inclusion(1, one) == "Refuted"
    assert k.inclusion(one, k.leaf({"a": 3}), budget=1) == "ResourceLimit"
    assert k.inclusion(one, k.leaf({"a": 3})) == "Proven"
    unknown = k.leaf({"a": None})
    assert k.inclusion(unknown, one) == "Unknown"
    print("Map inclusions:", count, "; negative-map projections:", projections, "; zero oracle disagreements")


def sharing():
    # One instantiation map for the entire scheme, not per branch or polarity.
    variable = ("var", "A")
    scheme = ("union", ("rec", ("cell", ("invariant", variable))),
              ("rec", ("use", ("function", variable, variable))))
    memo, renaming = {}, {}
    def fresh(t):
        if t in memo: return memo[t]
        if t[0] == "var":
            out = renaming.setdefault(t, ("var", len(renaming)))
        else: out = tuple(fresh(x) if isinstance(x, tuple) else x for x in t)
        memo[t] = out
        return out
    out = fresh(scheme)
    assert out[1][1][1][1] is out[2][1][1][1] is out[2][1][1][2]
    assert len(renaming) == 1
    print("Invariant/covariant/contravariant sharing: one variable retained")


def scale():
    spec = importlib.util.spec_from_file_location("demand", Path(__file__).with_name("t3-record-demand.py"))
    demand = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(demand)
    demand.scale()  # Suspensions, exact graph wire format and Fan(Never).
    start = perf_counter()
    k = Kernel()
    fan = 1
    for i in range(20):
        fan = k.op("and", fan, k.op("or", k.leaf({f"a{i}": 1}), k.leaf({f"b{i}": 3})))
    assert k.inclusion(fan, 1) == "Proven"
    assert not k.empty(fan)
    elapsed = perf_counter() - start
    assert elapsed < .1, elapsed
    print("Demanded 20-step closed Fan BDD:", k.reachable(fan), "reachable nodes;", round(elapsed, 6), "seconds")


if __name__ == "__main__":
    scale()
    sharing()
    check_boolean()
    check_records()
