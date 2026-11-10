import wasmtime, struct, json
from pathlib import Path
root=Path('/tmp/review-num')
rows=[]
for level in [0,2,3]:
 for reply in [2.0,float('nan'),float('inf'),-float('inf')]:
  cfg=wasmtime.Config()
  cfg.wasm_gc=True;cfg.wasm_function_references=True;cfg.wasm_tail_call=True;cfg.wasm_exceptions=True
  engine=wasmtime.Engine(cfg);store=wasmtime.Store(engine)
  mod=wasmtime.Module.from_file(engine,str(root/f'async-o{level}.wasm'))
  ids=[];messages=[]
  def panic(caller,addr,n):
   messages.append(bytes(caller.get('memory').read(caller,addr,addr+n)).decode())
  imports=[]
  for imp in mod.imports:
   if imp.module=='review:pause':
    imports.append(wasmtime.Func(store,imp.type,lambda request_id: ids.append(request_id)))
   elif imp.name=='panic': imports.append(wasmtime.Func(store,imp.type,panic,access_caller=True))
   elif imp.name in ['reference_id','cancel']: imports.append(wasmtime.Func(store,imp.type,lambda *args: 0 if imp.name=='reference_id' else None))
   else: raise AssertionError((imp.module,imp.name))
  exp=wasmtime.Instance(store,mod,imports).exports(store)
  exp['ilang_start'](store)
  status=exp['asyncprobe/main.work'](store,-0.0)
  assert status==1 and len(ids)==1
  resume=next(k for k in exp if k.startswith('ilang_resume:'))
  try:
   status=exp[resume](store,ids[0],reply)
   assert status==0 and reply==2.0
   result=exp['ilang_result:asyncprobe/main.work'](store)
   assert struct.pack('d',result)==struct.pack('d',-0.0)
   outcome='negative zero bits preserved'
  except wasmtime.Trap:
   assert reply!=2.0 and len(messages)==1 and messages[0].startswith('host: value is not correct'),messages
   outcome=messages[0]
  rows.append({'level':level,'reply':str(reply),'outcome':outcome})
print(json.dumps(rows,indent=2))
(root/'async-results.json').write_text(json.dumps(rows,indent=2))
