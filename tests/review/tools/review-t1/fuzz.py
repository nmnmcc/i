from probes import make,BIN,ROOT
import random,subprocess,json
random.seed(412)
vals=['a','ab','b','c','']
ps=[('"a"',lambda v:v=='a'),('"ab"',lambda v:v=='ab'),('"b"',lambda v:v=='b'),('_: String - "a"',lambda v:v!='a'),('_: String - "ab" - "b"',lambda v:v not in ['ab','b']),('"a" <> _',lambda v:v.startswith('a')),('_',lambda v:True),('_: "a" | "b"',lambda v:v in ['a','b'])]
bs=[('True',lambda b:b),('False',lambda b:not b),('_',lambda b:True)]
rows=[]
for i in range(180):
 pats=[(random.choice(ps),random.choice(bs),random.random()<.12) for _ in range(random.randint(2,9))]
 expected={}
 for v in vals:
  for b in [True,False]:
   expected[v,b]=next((j for j,(p,q,g) in enumerate(pats) if p[1](v) and q[1](b) and not g),None)
 covered=all(v is not None for v in expected.values())
 source='f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {\n'+''.join('('+p[0]+','+q[0]+')'+(' ? False' if g else '')+' -> '+str(j)+'\n' for j,(p,q,g) in enumerate(pats))+'} }\n'
 if covered:
  source+='@main\nmain = () {\n'+''.join('assert(f(('+json.dumps(v)+','+str(b)+')) == '+str(j)+')\n' for (v,b),j in expected.items())+'Nil\n}\n'
 p=make('fuzz-'+str(i),source);(p/'i.toml').write_text('name="probe"\n')
 r=subprocess.run([BIN,'check','--offline','--project',str(p)],text=True,capture_output=True,timeout=10)
 row=dict(i=i,covered=covered,exit=r.returncode,stderr=r.stderr)
 if (r.returncode==0)!=covered:print('MISMATCH',i,covered,r.returncode,r.stderr,flush=True)
 if r.returncode==0 and covered:
  for opt in [0,2]:
   rr=subprocess.run([BIN,'run','--opt',str(opt),'--offline','--project',str(p)],text=True,capture_output=True,timeout=15)
   row['run'+str(opt)]=dict(exit=rr.returncode,stderr=rr.stderr)
   if rr.returncode:print('RUNTIME',i,opt,rr.returncode,rr.stderr,flush=True)
 rows.append(row)
 (ROOT/'fuzz.json').write_text(json.dumps(rows,indent=2))
print('DONE',len(rows),'covering',sum(r['covered'] for r in rows),flush=True)
