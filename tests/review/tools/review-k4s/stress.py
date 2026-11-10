from probe import check
import json
rows=[]
for n in [64,128,256,512,1024]:
 src='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run(A,N) = (body: () -> A ! N) { body() }
f0 = () { run(() { use() }) }
'''+''.join(f'f{i} = () {{ run(() {{ f{i-1}() }}) }}\n' for i in range(1,n))
 r=check('chain_'+str(n),src,timeout=20);rows.append({k:r[k] for k in ['name','time','code']})
 if r['code']=='TIMEOUT': break
for n in [32,64,128,256,512]:
 src=''.join(f'S{i}(N) {{ S{i}(() -> Nil ! N, (N) -> Nil) }}\nu{i}(N) = (s ! S{i}(N)) {{ S{i}(f, _) = s f() }}\n' for i in range(n))
 src+='run(A,N) = (body: () -> A ! N) { body() }\nwide = () { '+ ' '.join(f'run(() {{ u{i}() }})' for i in range(n)) + ' }\n'
 r=check('wide_'+str(n),src,timeout=20);rows.append({k:r[k] for k in ['name','time','code']})
 if r['code']=='TIMEOUT': break
open('/tmp/review-k4s/stress-summary.json','w').write(json.dumps(rows,indent=2))
