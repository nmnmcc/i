import json,pathlib,subprocess,os
r=pathlib.Path('/tmp/review-a1'); assertions={1:'f() == 42',3:'f() == "2"',8:'f() == 42',19:'f() == 42',22:'f() == 42',24:'f((x) { x + 1 }) == 42',27:'f() == 42',28:'f() == 42',32:'f() == Stop',34:'f(42) == 7',36:'f(42) == 42 && f("s") == 0',40:'f(0) == 42 && f(1) == 42 && f(2) == 7',49:'f(0) == 10 && f(1) == 20',50:'f(0) == 42 && f(1) == 7',52:'f((0, 42)) == 42 && f((42, 0)) == 42',57:'f(0) == 42 && f(1) == 42 && f(2) == 7',59:'outer("s") == (42, "s")',60:'outer("s") == "s"',64:'f == f',65:'f() == "yes"',66:'g() == 4'}
results=[]
for c in json.loads((r/'corpus.json').read_text()):
 if c['id'] not in assertions: continue
 d=r/'runtime-new'/f"{c['id']:02d}";(d/'src').mkdir(parents=True,exist_ok=True);(d/'i.toml').write_text('name = "app"\n');src=d/'src/main.i';src.write_text(c['source']+'\n@main\nmain = () { assert('+assertions[c['id']]+') Nil }\n');(d/'original.i').write_text(src.read_text())
 def run(cmd):
  p=subprocess.run([str(x) for x in cmd],capture_output=True,text=True,timeout=45,env={**os.environ,'REVIEW_RUN':'1'});return dict(rc=p.returncode,out=p.stdout,err=p.stderr)
 v={'id':c['id'],'assertion':assertions[c['id']],'before':run([r/'run-probe',d,'both',d/'before.wat']),'fmt':run([r/'target/debug/i','fmt',src]),'after':run([r/'run-probe',d,'both',d/'after.wat']),'idem':run([r/'target/debug/i','fmt','--check',src])};results.append(v);print(c['id'],v['before']['err'].strip(),v['after']['err'].strip(),v['fmt']['rc'],v['idem']['rc'],flush=True)
(r/'runtime-new-results.json').write_text(json.dumps(results,indent=2))
