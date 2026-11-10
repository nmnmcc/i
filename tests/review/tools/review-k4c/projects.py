from harness import *
import shutil
projects={'std':Path('/home/agent/Programming/i/.rd/std1/std'),'hello':Path('/home/agent/Programming/i/.rd/std1/examples/hello'),'bench':Path('/home/agent/Programming/i/.rd/std1/bench')}
for build in ['lane','redesign']:
 binary=ROOT/f'i-{build}'
 if not binary.exists():continue
 for name,p in projects.items():
  r=subprocess.run([str(binary),'check','--project',str(p),'--offline','--types'],capture_output=True,text=True,timeout=120)
  (ROOT/f'{build}-{name}.types').write_text(r.stdout)
  (ROOT/f'{build}-{name}.err').write_text(r.stderr)
  print(build,name,r.returncode,len(r.stdout.splitlines()),r.stderr[:300])
