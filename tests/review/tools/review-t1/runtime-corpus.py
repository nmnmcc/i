import subprocess,time,json
from pathlib import Path
root=Path('/home/agent/Programming/i/.rd/a');out=[]
for package,cmd in [('examples/hello','run'),('std','test'),('bench','test')]:
 for opt in [0,2]:
  t=time.monotonic()
  try:
   r=subprocess.run([str(root/'target/release/i'),cmd,'--project',str(root/package),'--offline','--opt',str(opt)],text=True,capture_output=True,timeout=180)
   row=dict(package=package,opt=opt,exit=r.returncode,seconds=time.monotonic()-t,stdout=r.stdout,stderr=r.stderr)
  except subprocess.TimeoutExpired as e:row=dict(package=package,opt=opt,exit='timeout',seconds=time.monotonic()-t)
  out.append(row);print(package,opt,row['exit'],round(row['seconds'],2),flush=True)
  Path('/tmp/review-t1/runtime-corpus.json').write_text(json.dumps(out,indent=2))
