#!/usr/bin/env python3
"""Finite, executable record-algebra design model; not compiler validation.

Run: python3 tests/t3-record-substitution.py
Grammar/bounds and results are described in records.md section 20.
The old mode keeps the common-field union and empty Rec nodes, but uses J8
for E4. The new mode uses exact unions and collapses plainly empty records.
New commutation compares normalized trees (stronger than set equality here).
Old commutation also compares independent exact closed-product denotations.
A separate finite-set interpreter checks the new rules on closed flat types.
"""
from functools import lru_cache
from itertools import combinations_with_replacement, product

N = ("never",)
I = ("int",)
ONE = ("lit", 1)
A = ("var",)
TOP = ("top",)
LABELS = ("a", "b", "c")


def rec(fields=()):
    return ("rec", tuple(sorted(fields)))


R0 = rec()


def union(*xs):
    return ("union", xs)


def inter(*xs):
    return ("inter", xs)


def without(t, labels):
    return ("without", t, tuple(sorted(labels)))


@lru_cache(None)
def inhabited(t):
    if t in (I, ONE, TOP):
        return True
    if t[0] == "rec":
        return all(not k.startswith("@") and inhabited(v) for k, v in t[1])
    if t[0] == "union":
        return any(inhabited(x) for x in t[1])
    return False


@lru_cache(None)
def empty(t):
    return t == N or (t[0] == "rec" and any(empty(v) for _, v in t[1]))


def combine(kind, parts, old):
    flat = []
    for p in parts:
        flat.extend(p[1] if p[0] == kind else (p,))
    if kind == "union":
        if TOP in flat:
            return TOP
        flat = [x for x in flat if x != N]
        if I in flat:
            flat = [x for x in flat if x != ONE]
        if old:
            rs = [dict(x[1]) for x in flat if x[0] == "rec"]
            if rs:
                fs = rs[0]
                for gs in rs[1:]:
                    fs = {k: combine("union", (fs[k], gs[k]), old)
                          for k in fs.keys() & gs.keys()}
                flat = [x for x in flat if x[0] != "rec"] + [rec(fs.items())]
    else:
        if N in flat:
            return N
        flat = [x for x in flat if x != TOP]
        # Only the new mode distributes intersections. The old witness uses
        # the installed rule: a union with a variable-like member stays.
        us = next((x for x in flat if x[0] == "union" and
                   (not old or all(y[0] in {"rec", "int", "lit"}
                                   for y in x[1]))), None)
        if us is not None:
            rest = list(flat)
            rest.remove(us)
            return combine("union", [combine("inter", rest + [x], old)
                                      for x in us[1]], old)
        rs = [x for x in flat if x[0] == "rec"]
        scalar = [x for x in flat if x in (I, ONE)]
        if rs and scalar:
            return N
        if ONE in scalar:
            flat = [x for x in flat if x != I]
        if rs:
            fs = {}
            for r in rs:
                for k, v in r[1]:
                    fs[k] = combine("inter", (fs[k], v), old) if k in fs else v
            r = rec(fs.items())
            if not old and empty(r):
                return N
            # Without already guarantees a record (E5 / record-head identity).
            if r == R0 and any(x[0] == "without" for x in flat):
                flat = [x for x in flat if x[0] != "rec"]
            else:
                flat = [x for x in flat if x[0] != "rec"] + [r]
    flat = sorted(set(flat))
    if not flat:
        return N if kind == "union" else TOP
    return flat[0] if len(flat) == 1 else (kind, tuple(flat))


def forget(t, labels, old):
    labels = frozenset(labels)
    if not labels:
        return combine("inter", (t, R0), old)  # E0
    if empty(t) or t in (I, ONE):
        return N  # E1
    if t == TOP:
        return R0
    if t[0] == "union":
        return combine("union", [forget(x, labels, old) for x in t[1]], old)
    if t[0] == "without":
        return forget(t[1], labels | set(t[2]), old)
    if t[0] == "rec":
        # J8 guard, including old mode: only discarded fields need witnesses.
        uncertain = any(k != label and (k.startswith("@") or label.startswith("@"))
                        for k, _ in t[1] for label in labels)
        if not uncertain and all(inhabited(v) for k, v in t[1] if k in labels):
            return rec((k, v) for k, v in t[1] if k not in labels)
    if t[0] == "inter" and R0 in t[1]:
        rest = [x for x in t[1] if x != R0]
        return forget(combine("inter", rest, old), labels, old)
    return without(t, labels)


@lru_cache(maxsize=250000)
def norm(t, old=False):
    k = t[0]
    if k == "rec":
        fs = {}
        for f, v in t[1]:
            v = norm(v, old)
            fs[f] = combine("inter", (fs[f], v), old) if f in fs else v
        r = rec(fs.items())
        return N if not old and empty(r) else r
    if k in ("union", "inter"):
        return combine(k, [norm(x, old) for x in t[1]], old)
    if k == "without":
        return forget(norm(t[1], old), t[2], old)
    return t


@lru_cache(maxsize=250000)
def subst(t, s):
    if t == A:
        return s
    if t[0] == "rec":
        return rec((k, subst(v, s)) for k, v in t[1])
    if t[0] in ("union", "inter"):
        return (t[0], tuple(subst(x, s) for x in t[1]))
    if t[0] == "without":
        return without(subst(t[1], s), t[2])
    return t


def records(values):
    return [rec(zip((k for k, x in zip(LABELS, xs) if x is not None),
                    (x for x in xs if x is not None)))
            for xs in product((None,) + values, repeat=3)]


MASKS = tuple(tuple(k for k, b in zip(LABELS, bits) if b)
              for bits in product((False, True), repeat=3))
CLOSED = records((N, ONE, I))
OPEN = records((N, ONE, I, A))


def cases():
    # All records of <=3 fields with leaves Never, 1, Int, or A; variable
    # A also occurs as a base. Every atom and unordered binary union receives
    # each of the eight removals. All open union trees also have their order
    # reversed and must normalize identically.
    atoms = OPEN + [A, N]
    for t in atoms:
        for ls in MASKS:
            yield "atom/remove", without(t, ls)
    for x, y in combinations_with_replacement(atoms, 2):
        for ls in MASKS:
            yield "union/remove", without(union(x, y), ls)
    # General spread/override and intersection with a record. More than the
    # reported conflict: inserted fields may themselves contain A.
    for r in OPEN:
        spread = inter(without(A, tuple(k for k, _ in r[1])), r)
        for ls in MASKS:
            yield "spread/remove", without(spread, ls)
        for other in (rec((("a", N),)), rec((("b", ONE),)), R0):
            for ls in MASKS:
                yield "spread/union/remove", without(union(other, spread), ls)
        for first in MASKS:
            for second in MASKS:
                yield "nested/remove", without(without(r, first), second)


# Independent set interpreter: absence, 1, another Int, and a non-Int.
# This is complete for the flat closed leaves used here, including extra fields.
ABSENT = None
ROWS = tuple(product((ABSENT, 1, 2, "other"), repeat=3))
ROW_INDEX = {r: i for i, r in enumerate(ROWS)}
ALL_ROWS = (1 << len(ROWS)) - 1
SCALARS = (1, 2, "other")


@lru_cache(None)
def den(t):
    """(scalar mask, record mask). No calls to norm or smart constructors."""
    if t == N:
        return (0, 0)
    if t == TOP:
        return (7, ALL_ROWS)
    if t == ONE:
        return (1, 0)
    if t == I:
        return (3, 0)
    k = t[0]
    if k == "rec":
        out = 0
        for i, row in enumerate(ROWS):
            if all(row[LABELS.index(f)] is not ABSENT and
                   den(v)[0] & (1 << SCALARS.index(row[LABELS.index(f)]))
                   for f, v in t[1]):
                out |= 1 << i
        return (0, out)
    if k in ("union", "inter"):
        a, b = (0, 0) if k == "union" else (7, ALL_ROWS)
        for x in t[1]:
            c, d = den(x)
            a, b = (a | c, b | d) if k == "union" else (a & c, b & d)
        return (a, b)
    if k == "without":
        base = den(t[1])[1]
        keep = [i for i, f in enumerate(LABELS) if f not in t[2]]
        rests = {tuple(row[i] for i in keep) for j, row in enumerate(ROWS)
                 if base & (1 << j)}
        return (0, sum(1 << j for j, row in enumerate(ROWS)
                       if tuple(row[i] for i in keep) in rests))
    raise AssertionError("set oracle requires a closed flat type")


def check_sets():
    count = 0
    old_wrong = 0
    for x, y in combinations_with_replacement(CLOSED, 2):
        for kind in ("union", "inter"):
            for ls in MASKS:
                t = without((kind, (x, y)), ls)
                assert den(norm(t)) == den(t), ("not an equality", t)
                old_wrong += den(norm(t, True)) != den(t)
                count += 1
    # E0..E5, and the tempting but false distribution over intersection.
    x, y = rec((("a", ONE),)), rec((("a", I),))
    for first in MASKS:
        for second in MASKS:
            t = without(without(union(x, y), first), second)
            assert den(norm(t)) == den(t)
            count += 1
    incompatible = rec((("a", N),))
    assert den(without(incompatible, ("a",))) == (0, 0)
    return count, old_wrong


@lru_cache(maxsize=250000)
def product_denotation(t):
    """Exact semantic key for the OLD mode's closed sum-of-products output.

    Independent of norm: collapse empty products; union scalar masks. Old
    normalization leaves at most one record per head even inside fields.
    The assertions ensure this key is never applied to a general record
    union. Distinct nonempty products have different sets: absence witnesses
    a different required label; a field value witnesses a different factor.
    """
    if t == N:
        return (0, None)
    if t == I:
        return (3, None)
    if t == ONE:
        return (1, None)
    if t[0] == "rec":
        fs = tuple((k, product_denotation(v)) for k, v in t[1])
        return (0, None) if any(v == (0, None) for _, v in fs) else (0, fs)
    if t[0] == "union":
        scalar, record = 0, None
        for child in t[1]:
            sc, rc = product_denotation(child)
            scalar |= sc
            if rc is not None:
                assert record is None or record == rc, ("not a product", t)
                record = rc
        return (scalar, record)
    raise AssertionError(("old closed result is outside the product grammar", t))


def reverse_unions(t):
    if t[0] == "rec":
        return rec((k, reverse_unions(v)) for k, v in t[1])
    if t[0] in ("union", "inter"):
        xs = tuple(reverse_unions(x) for x in t[1])
        return (t[0], xs[::-1] if t[0] == "union" else xs)
    if t[0] == "without":
        return without(reverse_unions(t[1]), t[2])
    return t


def rename(t, mapping):
    if t[0] == "rec":
        return rec((mapping.get(k, k), rename(v, mapping)) for k, v in t[1])
    if t[0] in ("union", "inter"):
        return (t[0], tuple(rename(x, mapping) for x in t[1]))
    if t[0] == "without":
        return without(rename(t[1], mapping), [mapping.get(k, k) for k in t[2]])
    return t


def check_labels():
    count = 0
    for x, y in product((N, ONE, I, R0), repeat=2):
        r = rec((("@K", x), ("a", y)))
        for ls in ((), ("a",), ("@K",), ("a", "@K")):
            t = without(union(r, rec((("@J", ONE),))), ls)
            for k, j in product(("a", "b", "c", "fresh"), repeat=2):
                mapping = {"@K": k, "@J": j}
                assert norm(rename(norm(t), mapping)) == norm(rename(t, mapping))
                count += 1
    # Individually inhabited fields need not remain jointly inhabited when
    # labels coincide. J8 must not use that as an erasure witness.
    r = rec((("@K", I), ("a", R0)))
    assert not inhabited(r)
    assert norm(rename(r, {"@K": "a"})) == N
    return count


def main():
    gone = without(union(rec((("a", N),)),
                         inter(without(A, ("b",)), rec((("b", N),)))), ("a",))
    before = norm(subst(norm(gone, True), R0), True)
    after = norm(subst(gone, R0), True)
    assert before == N and after == R0
    assert norm(subst(norm(gone), R0)) == norm(subst(gone, R0)) == N
    # A second witness: a common-field join loses a possibly empty field.
    latent = without(union(rec((("a", A),)), rec((("b", I),))), ("a",))
    assert norm(subst(norm(latent, True), rec((("c", N),))), True) == R0
    assert norm(subst(latent, rec((("c", N),)))) == rec((("b", I),))
    counts = {}
    failed_old = 0
    old_tree_differences = 0
    instances = 0
    for family, t in cases():
        counts[family] = counts.get(family, 0) + 1
        new_open, old_open = norm(t), norm(t, True)
        assert new_open == norm(reverse_unions(t)), ("order", t)
        for s in CLOSED:
            new_left = norm(subst(new_open, s))
            new_right = norm(subst(t, s))
            assert new_left == new_right, (family, t, s, new_left, new_right)
            old_left = norm(subst(old_open, s), True)
            old_right = norm(subst(t, s), True)
            old_tree_differences += old_left != old_right
            failed_old += product_denotation(old_left) != product_denotation(old_right)
            instances += 1
        if counts[family] % 2000 == 0:
            print("progress", family, counts[family], flush=True)
    set_count, old_set_failures = check_sets()
    label_count = check_labels()
    print("closed substitutions:", len(CLOSED))
    print("type families:", counts)
    print("types:", sum(counts.values()))
    print("substitution checks:", instances)
    print("old tree differences:", old_tree_differences)
    print("old semantic commutation failures:", failed_old)
    print("new commutation failures: 0")
    print("independent set checks:", set_count)
    print("old set-equality failures:", old_set_failures)
    print("union-order checks:", sum(counts.values()))
    print("label-substitution checks:", label_count)
    assert failed_old > 0 and old_set_failures > 0


if __name__ == "__main__":
    main()
