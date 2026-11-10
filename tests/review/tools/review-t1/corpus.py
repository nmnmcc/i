from pathlib import Path
import subprocess,json,time,hashlib
root=Path('/home/agent/Programming/i/.rd/a')
bins=[root/'target/t1/continued/i-baseline-94789e6',root/'target/release/i']
rows=[]
for p in [root/'examples/hello',root/'std',root/'bench']:
 for b in bins:
  t=time.monotonic();r=subprocess.run([str(b),'check','--offline','--types','--project',str(p)],capture_output=True,text=True)
  row=dict(project=str(p),binary=str(b),exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr);rows.append(row)
  print(p.name,b.name,r.returncode,round(row['seconds'],3),r.stderr[:200],flush=True)
Path('/tmp/review-t1/corpus.json').write_text(json.dumps(rows,indent=2))
print('sha256',[(b.name,hashlib.sha256(b.read_bytes()).hexdigest()) for b in bins])
