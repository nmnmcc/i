from probe import write
import subprocess,time,json
exe='/tmp/review-k4s/target/debug/review-k4s'
rows=[]
for generic in [False,True]:
 for n in ([64,128,256,512,1024] if not generic else [32,64,128,256,512]):
  if generic:
   p='/tmp/review-k4s/wide_'+str(n)
  else:
   src=''.join(f'S{i}(A) {{ S{i}(A) }}\nu{i} = (s ! S{i}(Int)) {{ Nil }}\n' for i in range(n))
   src+='run(A,N) = (body: () -> A ! N) { body() }\nwide = () { '+ ' '.join(f'run(() {{ u{i}() }})' for i in range(n)) + ' }\n'
   p=write('wide_plain_'+str(n),src)
  for mode in ['before','after']:
   start=time.monotonic()
   try:
    r=subprocess.run([exe,mode,str(p),'2'],capture_output=True,text=True,timeout=45)
    out=r.stderr;row=[generic,n,mode,time.monotonic()-start,r.returncode,out];
   except subprocess.TimeoutExpired as e:row=[generic,n,mode,time.monotonic()-start,'TIMEOUT',str(e)]
   rows.append(row);print(row,flush=True)
  if rows[-1][4]=='TIMEOUT':break
open('/tmp/review-k4s/wide-bench-summary.json','w').write(json.dumps(rows,indent=2))
