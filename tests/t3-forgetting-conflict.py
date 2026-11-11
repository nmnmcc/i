#!/usr/bin/env python3
"""A finite witness to a T3 source conflict, not a compiler implementation test.

records §3 says a field of Never makes a record type empty. Section 4.5
joins record alternatives by their common labels. Section 4.3 E1 forgets
from a plainly empty record to Never; E2 says forgetting distributes over
union. Each rewrite is required to be an equality, independently of order.
The approved joint J8 guard is vacuous when no field is discarded.

The empty record alone witnesses unequal results. A one-value universe is
enough: no field can have a value of Never in any larger universe either.
"""

NEVER = frozenset()
UNIVERSE = frozenset({()})


def records(fields):
    """Spec 4.9.2 / records §3: at least the required fields."""
    return frozenset(
        r for r in UNIVERSE
        if all(label in dict(r) and dict(r)[label] in values
               for label, values in fields.items())
    )


def join_record_fields(left, right):
    """records §4.5: common labels, with the union of their field types."""
    return {label: left[label] | right[label] for label in left.keys() & right.keys()}


def forget(values, labels):
    """records §3: an existential source record, equal outside labels."""
    def rest(r):
        return tuple((label, value) for label, value in r if label not in labels)
    return frozenset(x for x in UNIVERSE if any(rest(x) == rest(y) for y in values))


# This second model uses only canonical Union nodes: one record plus one
# retained spread. It covers precisely the forms of the counterexample.
N = ("never",)
R0 = ("rec", ())


def rec(**fields):
    # records §3: keep an empty field; do not turn the Rec into Never.
    return ("rec", tuple(sorted(fields.items())))


def combine(kind, parts):
    flat = []
    for part in parts:
        flat.extend(part[1] if part[0] == kind else [part])
    fields = None
    others = []
    for part in flat:
        if part == N:
            if kind == "inter":
                return N
            continue
        if part[0] == "rec":
            new = dict(part[1])
            if fields is None:
                fields = new
            elif kind == "union":
                # Every common field in this witness has the same Never type.
                assert all(fields[k] == new[k] for k in fields.keys() & new.keys())
                fields = {k: fields[k] for k in fields.keys() & new.keys()}
            else:
                assert all(fields[k] == new[k] for k in fields.keys() & new.keys())
                fields.update(new)
        elif part not in others:
            others.append(part)
    if fields is not None:
        others.append(rec(**fields))
    if not others:
        assert kind == "union"
        return N
    return others[0] if len(others) == 1 else (kind, tuple(others))


def without(base, labels):
    if base == N:
        return N
    if base[0] == "rec":
        fields = dict(base[1])
        if N in fields.values():  # E1 precedes E4, including under J8.
            return N
        # The only surviving Rec in this witness is the empty record. E4's
        # inhabitance guard is vacuous; no discarded field has an open type.
        assert not fields
        return R0
    if base[0] == "union":
        return combine("union", [without(x, labels) for x in base[1]])
    if base[0] == "without":
        return without(base[1], labels | base[2])
    return ("without", base, labels)


def normalize(t, substitution):
    if t[0] == "var":
        return substitution.get(t[1], t)
    if t[0] == "rec":
        return rec(**{k: normalize(v, substitution) for k, v in t[1]})
    if t[0] in {"union", "inter"}:
        return combine(t[0], [normalize(x, substitution) for x in t[1]])
    if t[0] == "without":
        return without(normalize(t[1], substitution), t[2])
    assert t == N
    return N


def main():
    left = {"a": NEVER}
    right = {"b": NEVER}
    labels = {"a"}
    joined = join_record_fields(left, right)
    join_first = forget(records(joined), labels)
    distribute_first = forget(records(left), labels) | forget(records(right), labels)
    print("Closed source: Gone = { ..{ a: Never } | { b: Never }, -a }")
    print("Join first: common labels =", joined, "; empty record present =", () in join_first)
    print("E2 then E1: empty record present =", () in distribute_first)

    # Gone(A) = { ..{ a: Never } | { ..A, b: Never }, -a }
    inner = ("inter", (("without", ("var", "A"), frozenset({"b"})), rec(b=N)))
    generic = ("without", ("union", (rec(a=N), inner)), frozenset({"a"}))
    open_form = normalize(generic, {})
    normalize_first = normalize(open_form, {"A": R0})
    substitute_first = normalize(generic, {"A": R0})
    print("Canonical generic source: Gone(A) = { ..{ a: Never } | { ..A, b: Never }, -a }")
    print("Normalize, then A = {..}:", normalize_first)
    print("A = {..}, then normalize:", substitute_first)
    assert normalize_first == substitute_first, (
        "joint §6 requires substitution stability, but records §4.3 E1/E2 "
        "and §4.5 give Never versus {..}; the union before E2 is canonical"
    )
    assert join_first == distribute_first, (
        "records §4.3 E2 is not an equality under the required record join; "
        "the empty record witnesses {..} != Never"
    )


if __name__ == "__main__":
    main()
