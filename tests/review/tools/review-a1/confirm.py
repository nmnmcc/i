import pathlib,subprocess,json,os
r=pathlib.Path('/tmp/review-a1');binary=r/'target/debug/i';results=[]
for name,src,types in [('capture','import (g = app/types)\nf = () { case (x) -> Int { case g.Box(x) { g.Box(v) -> v } }(42) { _ -> 1 } * 10 }\n\n@main\nmain = () { assert(f() == 10) Nil }\n',True),('idempotence','f = () { case (x) -> Int { x }(42) { _ -> 42 } * 10 }\n\n@main\nmain = () { assert(f() == 420) Nil }\n',False)]:
 for mode in ['plain','from-old','keep-layout']:
  d=r/'confirmed'/f'{name}-{mode}';(d/'src').mkdir(parents=True,exist_ok=True);(d/'i.toml').write_text('name = "app"\n');p=d/'src/main.i';p.write_text(src);(d/'original.i').write_text(src)
  if types:(d/'src/types.i').write_text('export (Box(..))\nBox { Box(Int) }\n')
  def cmd(args):
   z=subprocess.run([str(binary),*map(str,args)],capture_output=True,text=True,timeout=40);return dict(rc=z.returncode,out=z.stdout,err=z.stderr)
  z={'name':name,'mode':mode,'before_check':cmd(['check','--project',d]),'before_run':cmd(['run','--opt','0','--project',d])}
  z['format']=cmd(['fmt',*(['--'+mode] if mode!='plain' else []),'--report',p]);z['once']=p.read_text();(d/'once.i').write_text(p.read_text());z['after_check']=cmd(['check','--project',d]);z['after_run']=cmd(['run','--opt','0','--project',d]);z['fmt_check']=cmd(['fmt','--check',p]);z['twice_command']=cmd(['fmt',p]);z['twice']=p.read_text();results.append(z);print(name,mode,'before',z['before_check']['rc'],z['before_run']['rc'],'fmt',z['format']['rc'],'after',z['after_check']['rc'],'idem',z['fmt_check']['rc'],flush=True)
(r/'confirm-results.json').write_text(json.dumps(results,indent=2))
