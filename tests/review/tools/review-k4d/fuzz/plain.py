import random,subprocess,os,sys,collections,re
L='/home/agent/Programming/i/.rd/std1/target/release/i'; B='/tmp/review-k4d/base-target/release/i'
HEAD='''Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
Svc(A) { Svc(f: () -> A, m: (A) -> Nil) }
ask(A) = (s ! Svc(A)) { s.f() }
Cov(A) { Cov(f: () -> A) }
askc(A) = (s ! Cov(A)) { s.f() }
Cb(N) { Cb(f: () -> Int ! N, m: (N) -> Nil) }
call(N) = (c: Cb(N)) {
  Cb(f, ..) = c
  f()
}
run(A, N) = (body: () -> A ! N) { body() }
twice(N) = (body: () -> Int ! N) { body() + body() }
with_int = (body) {
  ! Svc(Int) = Svc(() { 1 }, (_: Int) { Nil })
  body()
}
with_dep = (body) {
  ! Dep = Dep(5)
  body()
}
with_cov = (body, v) {
  ! Cov(_) = Cov(() { v })
  body()
}
pick = (a, b) { [a, b] }
'''
R=random.Random(int(sys.argv[1])); NPROG=int(sys.argv[2])
ATOMS=['1','get()','ask()','askc()','h0()','h1()','run(() { ask() })','twice(() { get() })','with_int(() { ask() })','with_dep(() { get() })','with_cov(() { askc() }, 3)','call(Cb(() { get() }, (_: Dep) { Nil }))','call(Cb(() { ask() }, (_) { Nil }))','with_int(() { run(() { ask() + 1 }) })','with_dep(() { with_int(() { get() + ask() }) })','run(h0)','twice(h1)']
ROWS=['Int','String','Dep','Never','_']
def e(): return ' + '.join(R.choice(ATOMS) for _ in range(R.randint(1,3)))
def st():
    c=R.random()
    if c<0.2: return '! Dep = Dep(1)'
    if c<0.45: return '! Svc(Int) = Svc(() { '+e()+' }, (_: Int) { Nil })'
    if c<0.55: return '! Svc(String) = Svc(() { "s" }, (_: String) { Nil })'
    if c<0.7: return '! Cov(Int) = Cov(() { '+e()+' })'
    if c<0.8: return f'l{R.randint(0,9)} = () {{ {e()} }}'
    if c<0.9: return f'p{R.randint(0,9)} = pick(() {{ {e()} }}, () {{ {e()} }})'
    return f'x{R.randint(0,9)} = {e()}'
stats=collections.Counter(); os.makedirs('/tmp/review-k4d/fuzz/bad',exist_ok=True)
d='/tmp/review-k4d/fuzz/pl%s'%sys.argv[1]; os.makedirs(d+'/src',exist_ok=True); open(d+'/i.toml','w').write('name = "app"\n')
def chk(b):
    try:
        r=subprocess.run([b,'check','--project',d,'--types'],capture_output=True,text=True,timeout=60); return r.returncode,r.stdout+r.stderr
    except subprocess.TimeoutExpired: return 'T',''
def runp(b,opt='0'):
    try:
        rr=subprocess.run([b,'run','--project',d,'--opt',opt],capture_output=True,text=True,timeout=20); return rr.returncode,rr.stdout+rr.stderr
    except subprocess.TimeoutExpired: return -1,'TIMEOUT'
for i in range(NPROG):
    hs=[f'h0 = () {{ {R.choice(ATOMS[:4]+ATOMS[6:16])} }}', f'h1 = () {{\n  '+'\n  '.join([st() for _ in range(R.randint(0,2))]+[R.choice(ATOMS[:5]+ATOMS[6:16])])+'\n}']
    ms=['! Dep = Dep(1)','! Svc(Int) = Svc(() { get() }, (_: Int) { Nil })','! Cov(Int) = Cov(() { 2 })']
    R.shuffle(ms); ms=ms[:R.randint(1,3)]
    for _ in range(R.randint(0,4)): ms.insert(R.randrange(len(ms)+1),st())
    used=[m.split(' = ')[0] for m in ms if re.match(r'^[lx]\d = ',m)]
    fin=e()+''.join(' + '+(u+'()' if u[0]=='l' else u) for u in set(used))
    src=HEAD+'\n'.join(hs)+'\n@main\nmain = () {\n  '+'\n  '.join(ms+[fin])+'\n  Nil\n}\n'
    open(d+'/src/main.i','w').write(src)
    lr,lo=chk(L); br,bo=chk(B)
    stats[f'lane={lr} base={br}']+=1
    if 'panicked' in lo: stats['PANIC']+=1; open(f'/tmp/review-k4d/fuzz/bad/panic_{sys.argv[1]}_{i}.i','w').write(src+'\n/*'+lo+'*/')
    if lr==0 and br==0 and lo!=bo: stats['types-differ']+=1; open(f'/tmp/review-k4d/fuzz/bad/typediff_{sys.argv[1]}_{i}.i','w').write(src+'\n/* LANE\n'+lo+'\nBASE\n'+bo+'*/')
    if lr!=0 and br==0:
        rc,oo=runp(B); k='base-ok-lane-rej:'+('base-fails-at-run' if (rc!=0 or 'trap' in oo) else 'BASE-RUNS-FINE'); stats[k]+=1
        open(f'/tmp/review-k4d/fuzz/bad/{"regress" if "FINE" in k else "fixed"}_{sys.argv[1]}_{i}.i','w').write(src+'\n/* LANE\n'+lo+'*/')
    if lr==0:
        for opt in ('0','2'):
            rc,oo=runp(L,opt)
            if rc!=0 or 'trap' in oo: stats['LANE-RUNFAIL']+=1; open(f'/tmp/review-k4d/fuzz/bad/run_{sys.argv[1]}_{i}.i','w').write(src+'\n// '+oo[:300]); break
print(sys.argv[1],dict(stats))
