from harness import *
for n in [32,128,512,1024]:
 for mode in ['plain','row','wide','wide_open']:
  if mode=='plain':
   src='f_0 = (f) { f() }\n'+'\n'.join(f'f_{i} = (f) {{ f_{i-1}(f) }}' for i in range(1,n))
  elif mode=='row':
   src='S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\nrun(N) = (body: () -> Nil ! N) { body() }\nf_0 = () { run(() { use() }) }\n'+'\n'.join(f'f_{i} = () {{ run(() {{ f_{i-1}() }}) }}' for i in range(1,n))
  else:
   src='\n'.join(f'S{i}(A) {{ S{i}(A) }}\nu_{i}(A) = (s ! S{i}(A)) {{ Nil }}' for i in range(n))+'\n'
   if mode=='wide_open':src+='run(N) = (body: () -> Nil ! N) { body() }\n'
   src+='wide = () {\n'+'\n'.join(f'run(() {{ u_{i}() }})' if mode=='wide_open' else f'u_{i}()' for i in range(n))+'\n}'
  for b in [False,True]:
   r=check(f'stress_{mode}_{n}',src,b,timeout=30)
   print(mode,n,'base' if b else 'lane',r['code'],round(r['seconds'],4),flush=True)
