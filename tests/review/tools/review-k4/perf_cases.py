from pathlib import Path
r=Path('/tmp/review-k4/perf')
for n in [64,128,256,512,1024,2048]:
 for kind in ['generic_chain','plain_chain','wide','wide_generic']:
  if kind=='generic_chain':
   src='S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\nrun(N) = (body: () -> Nil ! N) { body() }\nf_0 = () { run(() { use() }) }\n'
   src+=''.join(f'f_{i} = () {{ run(() {{ f_{i-1}() }}) }}\n' for i in range(1,n))
  elif kind=='plain_chain':
   src='run(N) = (body: () -> Nil ! N) { body() }\nf_0 = () { Nil }\n'
   src+=''.join(f'f_{i} = () {{ run(() {{ f_{i-1}() }}) }}\n' for i in range(1,n))
  elif kind=='wide':
   src=''.join(f'S{i} {{ S{i} }}\nf_{i} = (s ! S{i}) {{ Nil }}\n' for i in range(n))
   src+='wide = () { '+ ' '.join(f'f_{i}()' for i in range(n))+' }\n'
  else:
   src=''.join(f'S{i}(A) {{ S{i}(A) }}\nf_{i}(A) = (s ! S{i}(A)) {{ Nil }}\n' for i in range(n))
   src+='run(N) = (body: () -> Nil ! N) { body() }\nwide = () { '+ ' '.join(f'run(() {{ f_{i}() }})' for i in range(n))+' }\n'
  p=r/f'{kind}_{n}';(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name = "app"\n');(p/'src/main.i').write_text(src)
