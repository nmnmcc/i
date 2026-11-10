import subprocess,time,json,statistics
from probe import BIN,ROOT
out={}
for name in ['chain512','reversechain512','valuechain1024','valuechain2048','ordinarychain2048']:
 samples=[]
 for _ in range(3):
  start=time.monotonic();r=subprocess.run([BIN,'check','--project',str(ROOT/name)],capture_output=True,timeout=15);samples.append(time.monotonic()-start)
  assert r.returncode==0,(name,r.stderr)
 out[name]={'seconds':samples,'median':statistics.median(samples)}
 print(name,out[name],flush=True)
(ROOT/'timing.json').write_text(json.dumps(out,indent=2))
