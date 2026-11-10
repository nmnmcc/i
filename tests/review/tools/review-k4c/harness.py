from pathlib import Path
import subprocess,time,json,itertools
ROOT=Path('/tmp/review-k4c')
LANE=ROOT/'i-lane'
BASE=ROOT/'i-redesign'
def make(name,source):
 p=ROOT/'cases'/name
 (p/'src').mkdir(parents=True,exist_ok=True)
 (p/'i.toml').write_text('name = "app"\n')
 (p/'src/main.i').write_text(source+'\n')
 return p
def check(name,source,base=False,timeout=15):
 p=make(name,source)
 t=time.monotonic()
 try:
  r=subprocess.run([str(BASE if base else LANE),'check','--project',str(p),'--types','--offline'],capture_output=True,text=True,timeout=timeout)
  result=dict(code=r.returncode,out=r.stdout,err=r.stderr,seconds=time.monotonic()-t)
 except subprocess.TimeoutExpired:
  result=dict(code='timeout',seconds=time.monotonic()-t)
 (p/('base.json' if base else 'lane.json')).write_text(json.dumps(result,indent=2))
 return result
def run(name,opt):
 p=ROOT/'cases'/name
 r=subprocess.run([str(LANE),'run','--project',str(p),'--offline','--opt',str(opt)],capture_output=True,text=True,timeout=40)
 (p/f'run{opt}.json').write_text(json.dumps(dict(code=r.returncode,out=r.stdout,err=r.stderr),indent=2))
 return r.returncode,r.stdout,r.stderr
if __name__=='__main__':
 cases={
 'original':'S(N) { S(() -> Nil ! N) }\nuse(N) = (s ! S(N)) { S(f) = s f() }\n@main\nmain(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }',
 'invariant':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\nwrap(N) = () { local: () -> Nil ! N | S(N) = () { use() } local() }',
 'two_env':'S(A, B) { S(() -> Nil ! A, () -> Nil ! B) }\nuse(A, B) = (s ! S(A, B)) { S(f, g) = s f() g() }\nwrap(A,B) = () { local: () -> Nil ! A | B | S(A, B) = () { use() } local() }',
 'mutual':'S(N) { S(() -> Nil ! N) }\na(N) = (b: Bool, s ! S(N)) { b |> { True -> { S(f) = s f() } False -> z(True) } }\nz = (b: Bool) { a(b) }',
 'recursive':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\nbad(N) = () { ! S(N) = S(() { use() }, (_: N) { Nil }) use() }',
 'self_apply':'bad = (x) { x(x) }',
 'recursive_local':'bad = () { f = (x) { x(x) } Nil }',
 'recursive_tuple':'bad = (x) { local = () { (x, x) } bad(local()) }',
 'closure_statement':'Dep { Dep(value: Int) }\nget = (d ! Dep) { d.value }\nS(N) { S(() -> Int ! N) }\nuse(N) = (s ! S(N)) { S(f) = s f() }\nrun(N) = (f: () -> Int ! N) { f() }\n@main\nmain = () { local = () { ! S(Dep) = S(() { get() }) run(() { use() }) } local() }',
 'closure_statement_ok':'Dep { Dep(value: Int) }\nget = (d ! Dep) { d.value }\nS(N) { S(() -> Int ! N) }\nuse(N) = (s ! S(N)) { S(f) = s f() }\nrun(N) = (f: () -> Int ! N) { f() }\n@main\nmain = () { ! Dep = Dep(42) local = () { ! S(Dep) = S(() { get() }) run(() { use() }) } local() Nil }',
 }
 for name,src in cases.items():
  r=check(name,src);print(name,r)
