from pathlib import Path
r=Path('/tmp/review-k4/cases')
base='''S(N) { S(() -> Nil ! N, (N) -> Nil) }
use(N) = (s ! S(N)) { S(f, _) = s f() }
run = (f) { f() }
id = (x) { x }
'''
wrappers=['use() use()', 'run(() { use() }) use()', 'use() run(() { use() })', 'f = use f()', 'f = () { use() } f()', 'id(use)()', 'run(id(use))','f = () { use() } run(f) run(f)', 'f = () { run(() { use() }) } f()', 'f = (use, use) f.0() f.1()', 'True |> { True -> use() False -> use() }', 'run(() { use() use() })']
for i,body in enumerate(wrappers):
 for anno in ['', '-> Nil ! N | S(N)']:
  for service in ['', 's ! S(N)']:
   src=base+f'f(N) = ({service}) {anno} {{ {body} }}\n@main\nmain = () {{ ! S(Never) = S(() {{ Nil }}, (_: Never) {{ Nil }}) f() }}\n'
   p=r/f'wrapper_{i}_{bool(anno)}_{bool(service)}';(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name = "app"\n');(p/'src/main.i').write_text(src)
