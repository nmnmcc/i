import subprocess,sys,time,json
from pathlib import Path
root=Path('/tmp/review-k4')
binary=sys.argv[1]
label=sys.argv[2]
results=[]
for p in sorted((root/'cases').iterdir()):
 t=time.monotonic()
 try:
  r=subprocess.run([binary,'check','--project',str(p),'--types'],text=True,capture_output=True,timeout=20)
  row=dict(case=p.name,code=r.returncode,time=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr)
 except subprocess.TimeoutExpired:
  row=dict(case=p.name,code='timeout',time=time.monotonic()-t)
 results.append(row)
 print(row['case'],row['code'],round(row['time'],3),row.get('stdout',''),row.get('stderr',''),flush=True)
(root/(label+'.json')).write_text(json.dumps(results,indent=2))
