import importlib.util,time
p='/home/agent/Programming/i/.rd/note-rec/tests/t3-record-substitution.py'
s=importlib.util.spec_from_file_location('m',p);m=importlib.util.module_from_spec(s);s.loader.exec_module(m)
r=m.R0
for i in range(1,17):
    a,b=f'a{i}',f'b{i}'
    raw=m.union(m.inter(m.without(r,(a,)),m.rec(((a,m.ONE),))),
                m.inter(m.without(r,(b,)),m.rec(((b,m.ONE),))))
    start=time.perf_counter();r=m.norm(raw);elapsed=time.perf_counter()-start
    count=len(r[1]);assert count==2**i
    assert all(len(x[1])==i for x in r[1])
    print(i,count,count*i,round(elapsed,4),flush=True)
# Compact input whose substitution collapses a would-be exponential DNF.
t=m.inter(*(m.union(m.rec(((f'a{i}',m.A),)),m.rec(((f'b{i}',m.I),))) for i in range(1,13)))
open_form=m.norm(t)
closed=m.norm(m.subst(t,m.N))
assert len(open_form[1])==4096 and closed[0]=='rec' and len(closed[1])==12
assert m.norm(m.subst(open_form,m.N))==closed
print('limit-order witness: 4096 open alternatives versus 1 after early substitution')
