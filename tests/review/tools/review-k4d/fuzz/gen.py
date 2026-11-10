import random,subprocess,os,sys,collections
L='/home/agent/Programming/i/.rd/std1/target/release/i'
HEAD='''Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
T(N) { T(f: () -> Int ! N, m: (N) -> Nil) }
us(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
ut(N) = (t ! T(N)) {
  T(f, ..) = t
  f()
}
usa(N) = (s ! S(N)) -> Int ! N | S(N) {
  S(f, ..) = s
  f()
}
run(A, N) = (body: () -> A ! N) { body() }
'''
R=random.Random(int(sys.argv[1])); NPROG=int(sys.argv[2])
def row(d=0):
    c=R.random()
    if d>2 or c<0.3: return 'Never'
    if c<0.5: return 'Dep'
    if c<0.65: return f'S({row(d+1)})'
    if c<0.8: return f'T({row(d+1)})'
    a,b=row(d+1),row(d+1)
    if a=='Never': return b
    if b=='Never' or a==b: return a
    return f'{a} | {b}'
def expr(helpers,d=0):
    n=R.randint(1,3); parts=[]
    for _ in range(n):
        c=R.random()
        if c<0.15: parts.append('1')
        elif c<0.3: parts.append('get()')
        elif c<0.45: parts.append('us()')
        elif c<0.55: parts.append('usa()')
        elif c<0.7: parts.append('ut()')
        elif c<0.85 and helpers: parts.append(R.choice(helpers)+'()')
        elif d<2: parts.append('run(() { '+expr(helpers,d+1)+' })')
        else: parts.append('2')
    return ' + '.join(parts)
def stmt(helpers,ind,d):
    c=R.random()
    if c<0.25: return ind+'! Dep = Dep(1)'
    k='S' if c<0.65 else 'T'
    r=row()
    return f'{ind}! {k}({r}) = {k}(() {{ {expr(helpers,d+1)} }}, (_: {r}) {{ Nil }})'
def body(helpers,ind,d=0,locals_=None):
    out=[]; hs=list(helpers)
    for _ in range(R.randint(0,3)):
        if R.random()<0.3 and d<2:
            name=f'l{d}_{len(out)}'
            out.append(f'{ind}{name} = () {{\n{body(hs,ind+"  ",d+1)}\n{ind}}}')
            hs.append(name)
        else: out.append(stmt(hs,ind,d))
    out.append(ind+expr(hs,d))
    return '\n'.join(out)
stats=collections.Counter()
os.makedirs('/tmp/review-k4d/fuzz/bad',exist_ok=True)
d='/tmp/review-k4d/fuzz/w%s'%sys.argv[1]; os.makedirs(d+'/src',exist_ok=True); open(d+'/i.toml','w').write('name = "app"\n')
for i in range(NPROG):
    helpers=[]; src=HEAD
    for h in range(R.randint(0,3)):
        name=f'h{h}'; src+=f'{name} = () {{\n{body(helpers,"  ")}\n}}\n'; helpers.append(name)
    src+=f'@main\nmain = () {{\n{body(helpers,"  ")}\n  Nil\n}}\n'
    open(d+'/src/main.i','w').write(src)
    try:
        r=subprocess.run([L,'check','--project',d],capture_output=True,text=True,timeout=60)
    except subprocess.TimeoutExpired:
        stats['check-timeout']+=1; open(f'/tmp/review-k4d/fuzz/bad/ctimeout_{sys.argv[1]}_{i}.i','w').write(src); continue
    o=r.stdout+r.stderr
    if 'panicked' in o: stats['PANIC']+=1; open(f'/tmp/review-k4d/fuzz/bad/panic_{sys.argv[1]}_{i}.i','w').write(src); continue
    if r.returncode!=0:
        import re
        for c in set(re.findall(r'error\[(E\d+)\]',o)): stats['rej-'+c]+=1
        stats['rejected']+=1; continue
    stats['accepted']+=1
    for opt in ('0','2'):
        try:
            rr=subprocess.run([L,'run','--project',d,'--opt',opt],capture_output=True,text=True,timeout=20); oo=rr.stdout+rr.stderr; rc=rr.returncode
        except subprocess.TimeoutExpired: oo='TIMEOUT'; rc=-1
        if rc!=0 or 'trap' in oo:
            stats['RUNFAIL']+=1; open(f'/tmp/review-k4d/fuzz/bad/run_{sys.argv[1]}_{i}_o{opt}.i','w').write(src+'\n// '+oo[:300].replace('\n','\n// ')); break
print(sys.argv[1],dict(stats))
