import subprocess,json,time
from pathlib import Path
r=Path('/tmp/review-k4');rows=[]
for i in range(6):
 for label in (['before','after'] if i%2==0 else ['after','before']):
  t=time.monotonic();p=subprocess.run([str(r/f'check-{label}'),'--project','/home/agent/Programming/i/.rd/std1/std','--time'],capture_output=True,text=True,timeout=60)
  row=dict(label=label,i=i,wall=time.monotonic()-t,code=p.returncode,stderr=p.stderr)
  rows.append(row);print(row,flush=True);(r/'std-perf-results.json').write_text(json.dumps(rows,indent=2))
