from pathlib import Path
import subprocess,json,time
r=Path('/tmp/review-k4');rows=[]
for name in ['finite','multi','mutual','closure_closed','matrix_True_False_True','mutual_True_True_True']:
 for opt in [0,2]:
  t=time.monotonic()
  try:
   p=subprocess.run([str(r/'i-after'),'run','--project',str(r/'cases'/name),'--opt',str(opt)],text=True,capture_output=True,timeout=30)
   row=dict(case=name,opt=opt,code=p.returncode,stdout=p.stdout,stderr=p.stderr,wall=time.monotonic()-t)
  except subprocess.TimeoutExpired:row=dict(case=name,opt=opt,code='timeout',wall=time.monotonic()-t)
  rows.append(row);print(row,flush=True);(r/'runtime-results.json').write_text(json.dumps(rows,indent=2))
