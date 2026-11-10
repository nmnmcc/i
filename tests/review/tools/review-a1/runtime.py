import pathlib,json,subprocess,os
r=pathlib.Path('/tmp/review-a1'); tests={2:'f() == "42"',6:'f() == 1',7:'f() == 42',12:'f() == 84',13:'f() == 42',14:'f() == 42',15:'f() == 42',16:'f() == 42',17:'f() == 42',18:'f()(1) == 2',20:'f() == 42',21:'f() == 42',25:'f((x) { x + 1 }) == 42',37:'f(42) == 42',39:'f(Ok(42)) == 42 && f(Error(7)) == 7',41:'f(0) == 10 && f(1) == 20',42:'f(0) == 11 && f(1) == 12',43:'f(1) == 42 && f(0) == 7',44:'f() == 42',45:'f(0) == 41 && f(1) == 42',46:'f(0) == 42 && f(1) == 7',47:'f(0) == "yes" && f(1) == "no"',51:'f(0) == 1 && f(-1) == 2 && f(3) == 3',54:'f() == -2',55:'f() == 38',56:'f(0) == 42 && f(1) == 42 && f(2) == 7',58:'f() == (42, "s")',61:'f() == 42'}
results=[]
for c in json.loads((r/'corpus.json').read_text()):
 if c['id'] not in tests:continue
 d=r/'runtime'/f"{c['id']:02d}";(d/'src').mkdir(parents=True,exist_ok=True);(d/'i.toml').write_text('name = "app"\n');src=c['source']+'\n@main\nmain = () { assert('+tests[c['id']]+') Nil }\n';(d/'src/main.i').write_text(src);(d/'original.i').write_text(src)
 def run(mode):
  p=subprocess.run([str(r/'run-probe'),str(d),mode,str(d/(mode+'.wat'))],env={**os.environ,'REVIEW_RUN':'1'},capture_output=True,text=True,timeout=45);return dict(rc=p.returncode,out=p.stdout,err=p.stderr)
 result={'id':c['id'],'assertion':tests[c['id']],'before':run('old')}
 p=subprocess.run([str(r/'syntax-probe')],input=json.dumps({'source':src,'old':True})+'\n',capture_output=True,text=True);fmt=json.loads(p.stdout)['format'];result['format']=fmt
 if 'out' in fmt:
  (d/'src/main.i').write_text(fmt['out']);result['after']=run('both')
 results.append(result);(r/'runtime-results.json').write_text(json.dumps(results,indent=2));print(c['id'],result['before']['err'].strip(), result.get('after',{}).get('err','REFUSED').strip(),flush=True)
