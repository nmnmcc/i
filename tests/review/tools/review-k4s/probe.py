from pathlib import Path
import subprocess,time,json
ROOT=Path('/tmp/review-k4s')
CLI='/home/agent/Programming/i/.rd/std1/target/debug/i'
def write(name,src):
 p=ROOT/name;(p/'src').mkdir(parents=True,exist_ok=True)
 (p/'i.toml').write_text('name = "app"\n')
 (p/'src/main.i').write_text(src+'\n');return p

def check(name,src,timeout=15):
 p=write(name,src)
 start=time.monotonic()
 try:r=subprocess.run([CLI,'check','--project',str(p),'--types','--offline'],capture_output=True,text=True,timeout=timeout);out=r.stdout+r.stderr;code=r.returncode
 except subprocess.TimeoutExpired as e:out=str(e);code='TIMEOUT'
 result=dict(name=name,code=code,time=time.monotonic()-start,out=out)
 (ROOT/(name+'.json')).write_text(json.dumps(result,indent=2))
 print(name,code,round(result['time'],3));print(out)
 return result
if __name__=='__main__':
 check('simple', '''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (body: () -> A ! N) { body() }
forward(N) = (s ! S(N)) { run(() { use() }) }
''')
