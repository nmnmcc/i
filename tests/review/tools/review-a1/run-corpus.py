import pathlib,subprocess,json,re,shutil
r=pathlib.Path('/tmp/review-a1'); binary=r/'target/debug/i'; probe=r/'probe'
def run(cmd):
 try:
  p=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,timeout=45)
  return {'rc':p.returncode,'out':p.stdout,'err':p.stderr}
 except subprocess.TimeoutExpired:return {'rc':124,'out':'','err':'TIMEOUT'}
results=[]
for c in json.loads((r/'corpus.json').read_text()):
 d=r/'cases'/f"{c['id']:02d}_{c['name']}"; (d/'src').mkdir(parents=True,exist_ok=True)
 (d/'i.toml').write_text('name = "app"\n'); src=d/'src/main.i'; src.write_text(c['source']); (d/'original.i').write_text(c['source'])
 c['before']=run([probe,d,'both',d/'before.wat'])
 if c['kind']=='old': c['old']=run([probe,d,'old',d/'old.wat'])
 c['cli_check']=run([binary,'check','--offline','--types','--project',d])
 c['fmt']=run([binary,'fmt',*(['--from-old'] if c['kind']=='old' else []),'--report',src]); c['formatted']=src.read_text()
 if c['fmt']['rc']==0:
  c['after']=run([probe,d,'both',d/'after.wat'])
  c['idempotent']=run([binary,'fmt','--check',src])
  before=d/('old.wat' if c['kind']=='old' else 'before.wat')
  if before.exists() and (d/'after.wat').exists():
   for side,wat in [('wat-a',before),('wat-b',d/'after.wat')]:
    (d/side).mkdir(exist_ok=True);shutil.copy(wat,d/side/'test.wat')
   c['wat_diff']=run(['python3','-I','scripts/wat-diff',d/'wat-a',d/'wat-b'])
 results.append(c)
 (r/'results.json').write_text(json.dumps(results,indent=2))
 print(c['id'],c['name'],'check',c['cli_check']['rc'],'fmt',c['fmt']['rc'],'idem',c.get('idempotent',{}).get('rc'), 'wat',c.get('wat_diff',{}).get('rc'),flush=True)
