import importlib.util, random, time
from itertools import product
p='/home/agent/Programming/i/.rd/note-rec/tests/t3-record-substitution.py'
spec=importlib.util.spec_from_file_location('model',p)
m=importlib.util.module_from_spec(spec); spec.loader.exec_module(m)
B=('var','B')
def subst2(t,a,b):
    if t==m.A: return a
    if t==B: return b
    if t[0]=='rec': return m.rec((k,subst2(v,a,b)) for k,v in t[1])
    if t[0] in ('union','inter'): return (t[0],tuple(subst2(x,a,b) for x in t[1]))
    if t[0]=='without': return m.without(subst2(t[1],a,b),t[2])
    return t
# Independent finite semantics, with four labels and a nested record value.
# A tagged representation prevents record/scalar collisions.
labels=('a','b','c','d')
scalars=(('i',1),('i',2),('s','x'))
inner=(('r',()),('r',(('a',scalars[0]),)),('r',(('b',scalars[2]),)))
vals=scalars
rows=tuple(('r',tuple((k,v) for k,v in zip(labels,vs) if v is not None))
           for vs in product((None,)+vals,repeat=4))
universe=scalars+rows
from functools import lru_cache
@lru_cache(None)
def denote(t):
    if t==m.N: return frozenset()
    if t==m.I: return frozenset(scalars[:2])
    if t==m.ONE: return frozenset(scalars[:1])
    if t==m.TOP: return frozenset(universe)
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
# This universe includes enough nested representatives for these types and no
# Top field is used; keep an explicit old-rule negative control.
closed=[m.N,m.ONE,m.I,m.R0,m.rec((('a',m.N),)),m.rec((('a',m.ONE),)),m.rec((('b',m.I),))]
atoms=[m.A,B,m.rec((('a',m.A),)),m.rec((('b',B),)),
       m.rec((('c',m.A),('d',B))),m.rec((('a',m.rec((('b',B),))),))]
trees=[]
for x,y in product(atoms,repeat=2):
    for op in (m.union,m.inter):
        for ls in ((),('a',),('b','d'),labels):
            trees.append(m.without(op(x,y),ls))
for x,y in product(atoms,repeat=2):
    trees.append(m.without(m.inter(m.without(x,('a',)),m.rec((('a',y),))),('b',)))
def flat(t, in_field=False):
    if t[0]=='rec': return not in_field and all(flat(v,True) for _,v in t[1])
    if t[0] in ('union','inter'): return all(flat(x,in_field) for x in t[1])
    if t[0]=='without': return not in_field and flat(t[1])
    return True
checks=0; oracle=0
for t in trees:
    n=m.norm(t)
    for a,b in product(closed,repeat=2):
        raw=subst2(t,a,b)
        l=m.norm(subst2(n,a,b));r=m.norm(raw)
        assert l==r,('commutation',t,a,b,l,r)
        checks+=1
        # Independent oracle only on flat instances; nested commutation is structural.
        if flat(raw):
            assert denote(raw)==denote(r),('set equality',raw,r)
            oracle+=1
print('two-variable/four-label/nested sources',len(trees),'substitutions',checks,'set checks',oracle,flush=True)
old=m.without(m.union(m.rec((('a',m.N),)),m.rec((('b',m.N),))),('a',))
assert denote(old)!=denote(m.norm(old,True))
print('independent old-rule negative control: detected',flush=True)
# Explicit branch-spread family. No factoring is permitted by U1/U2.
r=m.R0
for n in range(1,15):
    k=f'f{n}'
    t=m.union(m.inter(m.without(r,(k,)),m.rec(((k,m.ONE),))),
              m.inter(m.without(r,(k,)),m.rec(((k,m.I),))))
    start=time.perf_counter();r=m.norm(t);elapsed=time.perf_counter()-start
    count=len(r[1]) if r[0]=='union' else 1
    assert count==2**n
    print('branch fields',n,'alternatives',count,'normalization seconds',round(elapsed,4),flush=True)
