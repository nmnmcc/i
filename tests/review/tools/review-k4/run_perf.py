import subprocess,time,json,sys
from pathlib import Path
r=Path('/tmp/review-k4');rows=[]
for kind in ['plain_chain','generic_chain','wide','wide_generic']:
 for n in [64,128,256,512,1024,2048]:
  for label in ['before','after']:
   if any(x['kind']==kind and x['label']==label and x['code']=='timeout' for x in rows):continue
   p=r/'perf'/f'{kind}_{n}';t=time.monotonic()
   try:
    c=subprocess.run([str(r/f'check-{label}'),'--project',str(p),'--time'],capture_output=True,text=True,timeout=20)
    row=dict(kind=kind,n=n,label=label,code=c.returncode,wall=time.monotonic()-t,stderr=c.stderr[-1000:])
   except subprocess.TimeoutExpired:
    row=dict(kind=kind,n=n,label=label,code='timeout',wall=time.monotonic()-t)
   rows.append(row);print(row,flush=True)
   (r/'perf-results.json').write_text(json.dumps(rows,indent=2))
