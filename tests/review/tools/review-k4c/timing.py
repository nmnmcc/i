from harness import *
import resource,statistics
p='/home/agent/Programming/i/.rd/std1/std'
results={'lane':[],'redesign':[]}
for i in range(14):
 for name in (['lane','redesign'] if i%2 else ['redesign','lane']):
  before=resource.getrusage(resource.RUSAGE_CHILDREN)
  t=time.monotonic()
  r=subprocess.run([str(ROOT/f'i-{name}'),'check','--project',p,'--offline'],stdout=subprocess.DEVNULL,stderr=subprocess.PIPE)
  after=resource.getrusage(resource.RUSAGE_CHILDREN)
  assert r.returncode==0,r.stderr
  results[name].append(dict(wall=time.monotonic()-t,cpu=after.ru_utime+after.ru_stime-before.ru_utime-before.ru_stime))
(ROOT/'timing.json').write_text(json.dumps(results,indent=2))
for name,rows in results.items():
 print(name, {k:dict(median=statistics.median(r[k] for r in rows[2:]),minimum=min(r[k] for r in rows[2:]),maximum=max(r[k] for r in rows[2:])) for k in ['cpu','wall']})
