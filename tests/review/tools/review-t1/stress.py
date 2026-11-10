from pathlib import Path
import subprocess,time,json,statistics,importlib.util
spec=importlib.util.spec_from_file_location('gen','/home/agent/Programming/i/.rd/a/tests/t1/gen-stress.py');gen=importlib.util.module_from_spec(spec);spec.loader.exec_module(gen)
ROOT=Path('/tmp/review-t1');B='/home/agent/Programming/i/.rd/a/target/release/i';BASE='/home/agent/Programming/i/.rd/a/target/t1/continued/i-baseline-94789e6';out=[]
for kind,sizes in [('keys',[250,500,1000,2000,5000,10000]),('pick',[250,500,1000,2000,5000]),('alias',[250,500,1000,2000]),('nested',[10,20,30,50]),('calls',[2500,5000])]:
 for n in sizes:
  p=ROOT/f'stress-{kind}-{n}';(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name="stress"\n');(p/'src/main.i').write_text(gen.source(kind,n))
  for b in ([BASE,B] if kind in ['keys','pick','calls'] else [B]):
   times=[];code=None;stderr=''
   for repeat in range(4):
    t=time.monotonic()
    try:r=subprocess.run([b,'check','--offline','--project',str(p)],capture_output=True,text=True,timeout=20);code=r.returncode;stderr=r.stderr
    except subprocess.TimeoutExpired:code='timeout'
    times.append(time.monotonic()-t)
    if code!=0:break
   row=dict(kind=kind,n=n,binary=b,times=times,median=statistics.median(times[1:] or times),exit=code,stderr=stderr);out.append(row)
   print(kind,n,Path(b).name,code,round(row['median'],4),stderr[:200],flush=True)
   (ROOT/'stress.json').write_text(json.dumps(out,indent=2))
