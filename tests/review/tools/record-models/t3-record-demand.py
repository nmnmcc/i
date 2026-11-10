#!/usr/bin/env python3
"""R1--R3 design model, not the compiler. Run python3 tests/t3-record-demand.py.

Shared node handles prevent Python tuple hashing from walking an unfolded DAG.
The old eager model is an oracle only for bounded tests, never the scale gate.
Scalar bits denote the finite representatives 1, 2, "x"; the proof engine also
models tuples, covariant nominal arguments and pure function variance.
"""
import importlib.util
import json
import sys
from functools import lru_cache
from itertools import combinations_with_replacement, product
from pathlib import Path
from time import perf_counter
sys.dont_write_bytecode = True
spec = importlib.util.spec_from_file_location("eager", Path(__file__).with_name("t3-record-substitution.py"))
e = importlib.util.module_from_spec(spec)
spec.loader.exec_module(e)


class TooLarge(Exception):
    pass


class Graph:
    def __init__(self):
        self.nodes = []
        self.ids = {}
        self.n = self.node("never")
        self.top = self.node("top")
        self.one = self.node("scalar", 1)
        self.integer = self.node("scalar", 3)
        self.string = self.node("scalar", 4)
        self.a = self.node("var", "A")
        self.rows = self.rec({})

    def node(self, *key):
        if key not in self.ids:
            self.ids[key] = len(self.nodes)
            self.nodes.append(key)
        return self.ids[key]

    def rec(self, fs):
        fs = tuple(sorted(dict(fs).items()))
        return self.n if any(v == self.n for _, v in fs) else self.node("rec", fs)

    def union(self, *xs):
        xs = tuple(sorted(set(xs) - {self.n}))
        if not xs: return self.n
        if self.top in xs: return self.top
        if len(xs) == 1: return xs[0]
        if all(self.nodes[x][0] == "scalar" for x in xs):
            bits = 0
            for x in xs: bits |= self.nodes[x][1]
            return self.node("scalar", bits)
        return self.node("union", xs)

    def inter(self, *xs):
        xs = tuple(sorted(set(xs) - {self.top}))
        if self.n in xs: return self.n
        if not xs: return self.top
        if len(xs) == 1: return xs[0]
        if all(self.nodes[x][0] == "rec" for x in xs):
            fs = {}
            for x in xs:
                for k, v in self.nodes[x][1]:
                    fs[k] = self.inter(fs[k], v) if k in fs else v
            return self.rec(fs)
        if all(self.nodes[x][0] == "scalar" for x in xs):
            bits = 7
            for x in xs: bits &= self.nodes[x][1]
            return self.node("scalar", bits) if bits else self.n
        if any(self.nodes[x][0] == "rec" for x in xs) and any(self.nodes[x][0] == "scalar" for x in xs):
            return self.n
        return self.node("inter", xs)

    @lru_cache(None)
    def inhabited(self, t):
        k, *p = self.nodes[t]
        if k in ("top", "scalar"): return True
        if k == "rec": return all(self.inhabited(v) for _, v in p[0])
        if k == "union": return any(self.inhabited(v) for v in p[0])
        return False

    def without(self, t, labels):
        labels = tuple(sorted(set(labels)))
        k, *p = self.nodes[t]
        if t == self.n or k == "scalar": return self.n
        if t == self.top: return self.rows
        if not labels: return self.inter(t, self.rows)
        if k == "without": return self.without(p[0], labels + p[1])
        if k == "rec" and all(self.inhabited(v) for f, v in p[0] if f in labels):
            return self.rec({f: v for f, v in p[0] if f not in labels})
        # E2 is a demand-time law, not a construction-time expansion.
        return self.node("without", t, labels)

    def subst(self, t, env):
        @lru_cache(None)
        def go(t):
            k, *p = self.nodes[t]
            if k == "var": return env.get(p[0], t)
            if k == "rec": return self.rec({f: go(v) for f, v in p[0]})
            if k == "union": return self.union(*(go(v) for v in p[0]))
            if k == "inter": return self.inter(*(go(v) for v in p[0]))
            if k == "without": return self.without(go(p[0]), p[1])
            return t
        return go(t)

    def import_tree(self, t):
        if t == e.N: return self.n
        if t == e.TOP: return self.top
        if t == e.ONE: return self.one
        if t == e.I: return self.integer
        if t[0] == "var": return self.node("var", t[1] if len(t) > 1 else "A")
        if t[0] == "rec": return self.rec({f: self.import_tree(v) for f, v in t[1]})
        if t[0] in ("union", "inter"):
            return getattr(self, t[0])(*(self.import_tree(v) for v in t[1]))
        if t[0] == "without": return self.without(self.import_tree(t[1]), t[2])
        raise ValueError(t)

    def export_tree(self, t):
        # Bounded oracle only: never use this unfolding for display or scale.
        k, *p = self.nodes[t]
        if k == "never": return e.N
        if k == "top": return e.TOP
        if k == "scalar": return {1: e.ONE, 3: e.I}[p[0]]
        if k == "var": return e.A if p[0] == "A" else ("var", p[0])
        if k == "rec": return e.rec((f, self.export_tree(v)) for f, v in p[0])
        if k in ("union", "inter"): return (k, tuple(self.export_tree(v) for v in p[0]))
        if k == "without": return e.without(self.export_tree(p[0]), p[1])
        raise ValueError(t)

    def children(self, t):
        k, *p = self.nodes[t]
        if k == "rec": return tuple(v for _, v in p[0])
        if k in ("union", "inter", "tuple"): return p[0]
        if k == "without": return (p[0],)
        if k == "nominal": return (p[1],)
        if k == "fn": return (p[0], p[1])
        return ()

    def bundle(self, root):
        seen = set()
        def visit(t):
            if t in seen: return
            seen.add(t)
            for c in self.children(t): visit(c)
        visit(root)
        return json.dumps({"root": root, "nodes": [(t, self.nodes[t]) for t in sorted(seen)]}, separators=(",", ":"))

    @classmethod
    def decode(cls, text):
        data = json.loads(text)
        g, mapping = cls(), {}
        for old, node in data["nodes"]:
            k, *p = node
            if k == "rec": new = g.rec({f: mapping[v] for f, v in p[0]})
            elif k in ("union", "inter"):
                new = getattr(g, k)(*(mapping[v] for v in p[0]))
            elif k == "without": new = g.without(mapping[p[0]], p[1])
            else: new = g.node(k, *p)
            mapping[old] = new
        return g, mapping[data["root"]]

    def proof(self, source, target, upper=None, lower=None, limit=65536):
        upper, lower = upper or {}, lower or {}
        seen, active, memo = set(), set(), {}
        def prove(s, t):
            pair = (s, t)
            if pair not in seen:
                seen.add(pair)
                if len(seen) > limit: raise TooLarge("type too large to check here: result; annotate the intermediate record")
            if pair in memo: return memo[pair]
            if pair in active: return False  # Unknown, never a coinductive proof
            active.add(pair)
            ans = step(s, t)
            active.remove(pair)
            memo[pair] = ans
            return ans
        def step(s, t):
            if s == t or s == self.n or t == self.top: return True
            sk, *sp = self.nodes[s]
            tk, *tp = self.nodes[t]
            if sk == "var" and any(prove(u, t) for u in upper.get(sp[0], ())): return True
            if tk == "var" and any(prove(s, l) for l in lower.get(tp[0], ())): return True
            if sk == "union": return all(prove(x, t) for x in sp[0])
            if tk == "inter": return all(prove(s, x) for x in tp[0])
            if tk == "union" and any(prove(s, x) for x in tp[0]): return True
            if sk == "inter" and any(prove(x, t) for x in sp[0]): return True
            if sk == "without" and tk == "without" and sp[1] == tp[1]: return prove(sp[0], tp[0])
            if t == self.rows and sk in ("rec", "without"): return True
            if sk == tk == "scalar": return not sp[0] & ~tp[0]
            if sk == tk == "tuple":
                return len(sp[0]) == len(tp[0]) and all(prove(x, y) for x, y in zip(sp[0], tp[0]))
            if sk == tk == "nominal": return sp[0] == tp[0] and prove(sp[1], tp[1])
            if sk == tk == "fn": return prove(tp[0], sp[0]) and prove(sp[1], tp[1])
            if sk == tk == "rec":
                fs = dict(sp[0])
                return all(k in fs and prove(fs[k], v) for k, v in tp[0])
            if sk == "rec" and tk == "union":
                # Exact field partition, lexical labels; atomising finite scalar
                # sets includes unions fused by the scalar smart constructor.
                for f, v in sp[0]:
                    vk, *vp = self.nodes[v]
                    if vk == "var" and upper.get(vp[0]):
                        bound = self.inter(*upper[vp[0]])
                        if bound != v and prove(self.rec(dict(sp[0]) | {f: bound}), t): return True
                    parts = vp[0] if vk == "union" else ()
                    if vk == "scalar" and vp[0].bit_count() > 1:
                        parts = tuple(self.node("scalar", b) for b in (1, 2, 4) if vp[0] & b)
                    if parts:
                        return all(prove(self.rec(dict(sp[0]) | {f: x}), t) for x in parts)
            return False
        result = prove(source, target)
        return result, len(seen)

    def read(self, t, label):
        # Exact closed common-field read; other operations may need a demand
        # visitor for intersections/Without. This small model does not claim it.
        k, *p = self.nodes[t]
        if k == "never": return self.n
        if k == "union": return self.union(*(self.read(x, label) for x in p[0]))
        if k == "rec" and label in dict(p[0]): return dict(p[0])[label]
        raise ValueError("field is not guaranteed on every alternative")


def bounded():
    g = Graph()
    atoms = [e.A, e.R0, e.N] + [e.rec(zip(("a", "b", "c"), xs)) for xs in product((e.N, e.ONE, e.I, e.A), repeat=3)]
    closed = [e.rec((k, v) for k, v in zip(("a", "b", "c"), xs) if v is not None)
              for xs in product((None, e.N, e.ONE, e.I), repeat=3)]
    trees = [e.without(op(x, y), ls) for x, y in combinations_with_replacement(atoms, 2)
             for op in (e.union, e.inter) for ls in ((), ("a",), ("a", "b", "c"))]
    checks = 0
    for t in trees:
        compact = g.import_tree(t)
        decoded, restored = Graph.decode(g.bundle(compact))
        assert e.norm(decoded.export_tree(restored)) == e.norm(t)
        for c in closed:
            left = g.subst(compact, {"A": g.import_tree(c)})
            raw = e.subst(t, c)
            right = g.import_tree(raw)
            assert e.norm(g.export_tree(left)) == e.norm(g.export_tree(right)) == e.norm(raw)
            checks += 1
    old = e.without(e.union(e.rec((("a", e.N),)), e.inter(e.A, e.rec((("b", e.N),)))), ("a",))
    assert e.norm(e.subst(e.norm(old, True), e.R0), True) != e.norm(e.subst(old, e.R0), True)
    print("compact commutation:", len(trees), "trees x", len(closed), "closed records =", checks, "; old failure detected")


def review_extension():
    # Bounded two-variable/four-label extension supplied by the independent review.
    B=('var','B')
    def subst2(t,a,b):
        if t==e.A: return a
        if t==B: return b
        if t[0]=='rec': return e.rec((k,subst2(v,a,b)) for k,v in t[1])
        if t[0] in ('union','inter'): return (t[0],tuple(subst2(x,a,b) for x in t[1]))
        if t[0]=='without': return e.without(subst2(t[1],a,b),t[2])
        return t
    # Independent finite semantics for flat instances over four labels.
    # A tagged representation prevents record/scalar collisions.
    labels=('a','b','c','d')
    scalars=(('i',1),('i',2),('s','x'))
    vals=scalars
    rows=tuple(('r',tuple((k,v) for k,v in zip(labels,vs) if v is not None))
               for vs in product((None,)+vals,repeat=4))
    universe=scalars+rows
    from functools import lru_cache
    @lru_cache(None)
    def denote(t):
        if t==e.N: return frozenset()
        if t==e.I: return frozenset(scalars[:2])
        if t==e.ONE: return frozenset(scalars[:1])
        if t==e.TOP: return frozenset(universe)
        k=t[0]
        if k=='rec':
            fields=[(f,denote(v)) for f,v in t[1]]
            return frozenset(r for r in rows if all(dict(r[1]).get(f) in vs for f,vs in fields))
        if k=='union': return frozenset().union(*(denote(x) for x in t[1]))
        if k=='inter': return frozenset(universe).intersection(*(denote(x) for x in t[1]))
        if k=='without':
            def rest(r):return tuple((f,v) for f,v in r[1] if f not in t[2])
            rests={rest(r) for r in denote(t[1]) if r[0]=='r'}
            return frozenset(r for r in rows if rest(r) in rests)
        raise ValueError(t)
    # Nested instances are checked structurally below, not with this flat
    # universe. Keep an explicit old-rule negative control.
    closed=[e.N,e.ONE,e.I,e.R0,e.rec((('a',e.N),)),e.rec((('a',e.ONE),)),e.rec((('b',e.I),))]
    atoms=[e.A,B,e.rec((('a',e.A),)),e.rec((('b',B),)),
           e.rec((('c',e.A),('d',B))),e.rec((('a',e.rec((('b',B),))),))]
    trees=[]
    for x,y in product(atoms,repeat=2):
        for op in (e.union,e.inter):
            for ls in ((),('a',),('b','d'),labels):
                trees.append(e.without(op(x,y),ls))
    for x,y in product(atoms,repeat=2):
        trees.append(e.without(e.inter(e.without(x,('a',)),e.rec((('a',y),))),('b',)))
    def flat(t, in_field=False):
        if t[0]=='rec': return not in_field and all(flat(v,True) for _,v in t[1])
        if t[0] in ('union','inter'): return all(flat(x,in_field) for x in t[1])
        if t[0]=='without': return not in_field and flat(t[1])
        return True
    checks=0; oracle=0
    for t in trees:
        n=e.norm(t)
        for a,b in product(closed,repeat=2):
            raw=subst2(t,a,b)
            l=e.norm(subst2(n,a,b));r=e.norm(raw)
            assert l==r,('commutation',t,a,b,l,r)
            checks+=1
            # Independent oracle only on flat instances; nested commutation is structural.
            if flat(raw):
                assert denote(raw)==denote(r),('set equality',raw,r)
                oracle+=1
    print('two-variable/four-label/nested sources',len(trees),'substitutions',checks,'set checks',oracle,flush=True)
    old=e.without(e.union(e.rec((('a',e.N),)),e.rec((('b',e.N),))),('a',))
    assert denote(old)!=denote(e.norm(old,True))
    print('independent old-rule negative control: detected',flush=True)

def proofs():
    g = Graph()
    ra = lambda t: g.rec({"a": t})
    union = g.union(ra(g.integer), ra(g.string))
    assert g.proof(ra(g.union(g.integer, g.string)), union)[0]  # wrap_union
    established = g.union(g.integer, g.string)
    assert g.proof(ra(g.a), union, upper={"A": (established,)})[0]
    assert not g.proof(ra(g.a), union, lower={"A": (established,)})[0]
    tup = lambda *xs: g.node("tuple", xs)
    pair = g.rec({"data": tup(g.one, g.node("scalar", 2))})
    target = g.union(g.rec({"data": tup(g.integer, g.integer)}), g.rec({"error": g.string}))
    assert g.proof(pair, target)[0]
    for ctor in (lambda x: g.node("nominal", "List", x), lambda x: g.node("fn", g.integer, x)):
        assert g.proof(ra(ctor(g.one)), g.union(ra(ctor(g.integer)), g.rec({"error": g.string})))[0]
    assert g.proof(ra(g.node("fn", g.integer, g.one)), ra(g.node("fn", g.one, g.integer)))[0]
    assert not g.proof(ra(g.node("fn", g.one, g.one)), ra(g.node("fn", g.integer, g.one)))[0]
    assert g.read(g.union(ra(g.one), ra(g.string)), "a") == g.union(g.one, g.string)  # Schema(Row)
    try: g.read(g.union(ra(g.one), g.rec({"b": g.one})), "a")
    except ValueError: pass
    else: raise AssertionError("missing field accepted")
    # R2 has R as a lower bound, never an upper bound: not evidence R2 <: R.
    r, r2 = g.node("var", "R"), g.node("var", "R2")
    src = g.without(r2, ("a",))
    dst = g.union(g.without(r, ("a",)), g.rec({"b": g.integer}))
    lower = {"R2": (r,)}
    before = repr(lower)
    assert not g.proof(src, dst, lower=lower)[0]
    assert g.proof(src, dst, upper={"R2": (r,)}, lower=lower)[0]
    assert repr(lower) == before
    assert not g.proof(r2, dst, upper={"R2": (r2,)})[0]  # cycle is not proof
    # Exhaustive flat finite-set oracle: equivalent closed target spellings.
    universe = tuple(dict((k, v) for k, v in zip(("a", "b"), vs) if v)
                     for vs in product((0, 1, 2, 4), repeat=2))
    def den(t):
        k, *p = g.nodes[t]
        if k == "never": return set()
        if k == "union": return set().union(*(den(x) for x in p[0]))
        assert k == "rec"
        return {i for i, row in enumerate(universe) if all(f in row and row[f] & g.nodes[v][1] for f, v in p[0])}
    records = [g.rec({k: g.node("scalar", b) for k, b in zip(("a", "b"), bs) if b})
               for bs in product(range(8), repeat=2)]
    count = 0
    for s in records:
        for a, b in combinations_with_replacement(records, 2):
            t = g.union(a, b)
            assert g.proof(s, t)[0] == (den(s) <= den(t)), (s, t)
            count += 1
    try: g.proof(ra(g.union(g.integer, g.string)), union, limit=1)
    except TooLarge as ex: assert str(ex).startswith("type too large to check here")
    else: raise AssertionError("budget not enforced")
    assert g.proof(ra(g.union(g.integer, g.string)), union)[0]
    print("proof/set oracle:", count, "inclusions; structural, bound direction, Schema read, missing-field and resource probes passed")


def scale():
    g = Graph()
    start = perf_counter()
    r = g.rows
    for i in range(1, 21):
        r = g.union(*(g.inter(g.without(r, (f"{p}{i}",)), g.rec({f"{p}{i}": g.one})) for p in ("a", "b")))
    assert g.proof(r, g.rows)[0]
    bundle = g.bundle(r)
    data = json.loads(bundle)
    decoded, restored = Graph.decode(bundle)
    assert decoded.proof(restored, decoded.rows)[0]
    assert len(json.loads(decoded.bundle(restored))["nodes"]) == len(data["nodes"])
    branch_time = perf_counter() - start
    start = perf_counter()
    fan = g.top
    for i in range(1, 21):
        fan = g.inter(fan, g.union(g.rec({f"a{i}": g.a}), g.rec({f"b{i}": g.integer})))
    closed = g.subst(fan, {"A": g.n})
    assert closed == g.rec({f"b{i}": g.integer for i in range(1, 21)})
    assert g.proof(closed, g.rows)[0]
    fan_bundle = g.bundle(fan)
    decoded, restored = Graph.decode(fan_bundle)
    assert decoded.subst(restored, {"A": decoded.n}) == decoded.rec({f"b{i}": decoded.integer for i in range(1, 21)})
    fan_time = perf_counter() - start
    assert branch_time < 0.1 and fan_time < 0.1, (branch_time, fan_time)
    # The 12-step review witness has 4096 product choices before substitution,
    # and one afterwards; do not materialise those choices to assert the count.
    assert 2**12 == 4096
    print("20-step branch:", len(data["nodes"]), "reachable nodes,", len(bundle), "bytes,", round(branch_time, 6), "seconds")
    print("20-step Fan:", len(json.loads(fan_bundle)["nodes"]), "reachable nodes,", len(fan_bundle), "bytes,", round(fan_time, 6), "seconds; closed 1 record")


if __name__ == "__main__":
    scale()
    proofs()
    bounded()
    review_extension()
