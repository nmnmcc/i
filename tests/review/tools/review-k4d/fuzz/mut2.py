import random,subprocess,os,sys,collections,re
L='/home/agent/Programming/i/.rd/std1/target/release/i'
exec(open('gen.py').read().split("R=random.Random")[0].split("HEAD='''")[0])
HEAD=open('gen.py').read().split("HEAD='''")[1].split("'''")[0]
R=random.Random(int(sys.argv[1])); NPROG=int(sys.argv[2])
ROWS=['Never','Dep','T(Dep)','T(Never)','S(Never)','S(Dep)','Dep | T(Dep)','Dep | S(Dep)','T(S(Never))','S(T(Dep))','Dep | T(Dep) | S(Dep | T(Dep))']
ATOMS=['1','get()','us()','usa()','ut()','h0()','h1()','run(() { us() })','run(() { ut() + get() })','run(() { run(() { usa() }) })']
def e(): return ' + '.join(R.choice(ATOMS) for _ in range(R.randint(1,3)))
def st():
    c=R.random()
    if c<0.2: return '! Dep = Dep(1)'
    k=R.choice('ST'); r=R.choice(ROWS)
    return f'! {k}({r}) = {k}(() {{ {e()} }}, (_: {r}) {{ Nil }})'
BASES=[
 (['h0 = () { run(() { us() }) + ut() }','h1 = () { usa() }'],
  ['! Dep = Dep(1)','! T(Dep) = T(() { get() }, (_: Dep) { Nil })','! S(Dep | T(Dep)) = S(() { ut() + get() }, (_: Dep | T(Dep)) { Nil })','l = () { run(() { us() + usa() }) }','h0() + l() + ut() + h1()']),
 (['h0 = () { us() }','h1 = () { run(h0) + run(() { ut() }) }'],
  ['! S(Never) = S(() { 1 }, (_: Never) { Nil })','! T(S(Never)) = T(() { us() + h0() }, (_: S(Never)) { Nil })','k = () { h1() }','k() + ut()']),
 (['h0 = () { get() + us() }','h1 = () {\n  ! Dep = Dep(2)\n  h0()\n}'],
  ['! S(Dep) = S(() { get() }, (_: Dep) { Nil })','a = h1()','! Dep = Dep(3)','! T(Dep | S(Dep)) = T(() { h0() }, (_: Dep | S(Dep)) { Nil })','a + ut() + run(() { ut() })']),
]
stats=collections.Counter()
os.makedirs('bad',exist_ok=True)
d='/tmp/review-k4d/fuzz/m%s'%sys.argv[1]; os.makedirs(d+'/src',exist_ok=True); open(d+'/i.toml','w').write('name = "app"\n')
for i in range(NPROG):
    hs,ms=R.choice(BASES); hs=list(hs); ms=list(ms)
    for _ in range(R.randint(0,3)):
        c=R.random()
        if c<0.2 and len(ms)>1: ms.pop(R.randrange(len(ms)-1))
        elif c<0.35 and len(ms)>2:
            j=R.randrange(len(ms)-2); ms[j],ms[j+1]=ms[j+1],ms[j]
        elif c<0.55: ms.insert(R.randrange(len(ms)),st())
        elif c<0.7:
            j=R.randrange(len(ms)); ms[j]=re.sub(r'\b(us|ut|usa|get|h0|h1)\(\)',lambda m:R.choice(ATOMS) if R.random()<0.4 else m.group(0),ms[j])
        elif c<0.85:
            j=R.randrange(len(hs)); hs[j]=re.sub(r'\b(us|ut|usa|get)\(\)',lambda m:R.choice(ATOMS[:5]) if R.random()<0.5 else m.group(0),hs[j])
        else:
            j=R.randrange(len(ms)-1)
            if ms[j].startswith('!'):
                r=R.choice(ROWS); ms[j]=re.sub(r'^! (\w)\((.*?)\) = (\w)\(\(\) \{(.*)\}, \(_: .*\) \{ Nil \}\)$',lambda m:f'! {m.group(1)}({r}) = {m.group(3)}(() {{{m.group(4)}}}, (_: {r}) {{ Nil }})',ms[j])
    if R.random()<0.3 and len(ms)>2:
        j=R.randrange(len(ms)-1); k=R.randrange(j,len(ms)-1)
        inner=ms[j:k+1]; 
        if not any(re.match(r'^\w+ = ',x) for x in inner):
            ms[j:k+1]=['w = () {\n    '+'\n    '.join(inner+[e()])+'\n  }']; ms[-1]+= ' + w()'
    src=HEAD+'\n'.join(hs)+'\n@main\nmain = () {\n  '+'\n  '.join(ms)+'\n  Nil\n}\n'
    open(d+'/src/main.i','w').write(src)
    try: r=subprocess.run([L,'check','--project',d],capture_output=True,text=True,timeout=60)
    except subprocess.TimeoutExpired:
        stats['check-timeout']+=1; open(f'bad/ctimeout_{sys.argv[1]}_{i}.i','w').write(src); continue
    o=r.stdout+r.stderr
    if 'panicked' in o: stats['PANIC']+=1; open(f'bad/panic_{sys.argv[1]}_{i}.i','w').write(src); continue
    if r.returncode!=0:
        cs=set(re.findall(r'error\[(E\d+)\]',o))
        if 'E0505' in cs: os.makedirs('e0505',exist_ok=True); open(f'e0505/{sys.argv[1]}_{i}.i','w').write(src+'\n// '+o.replace('\n','\n// '))
        for c in cs: stats['rej-'+c]+=1
        stats['rejected']+=1; continue
    stats['accepted']+=1
    for opt in ('0','2'):
        try:
            rr=subprocess.run([L,'run','--project',d,'--opt',opt],capture_output=True,text=True,timeout=20); oo=rr.stdout+rr.stderr; rc=rr.returncode
        except subprocess.TimeoutExpired: oo='TIMEOUT'; rc=-1
        if rc!=0 or 'trap' in oo:
            stats['RUNFAIL']+=1; open(f'bad/run_{sys.argv[1]}_{i}_o{opt}.i','w').write(src+'\n// '+oo[:300].replace('\n','\n// ')); break
print(sys.argv[1],dict(stats))
