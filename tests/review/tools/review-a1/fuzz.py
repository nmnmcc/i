import itertools,json,pathlib,subprocess
r=pathlib.Path('/tmp/review-a1'); cases=[]
subjects=['42','(1, 2)','(x) -> Int { x }(42)','(x) -> Int { x }','{ 1 + 2 }','1 == 2','"a" <> "b"','case 1 { _ -> 42 }']
patterns=['_','x','(a, b)','t = (a, b)','_: Int']
bodies=['42','(x) { x }','(x) -> Int { x }','(x) -> Int { x }(42)','{ g = (x) -> Int { x } g }']
contexts=['f = () { EXPR }','f = () { EXPR * 10 }','f = () { 10 + EXPR }','f = () { EXPR(1) }','f = () { { EXPR } }','f = () { [EXPR] }']
for s,p,b,ctx in itertools.product(subjects,patterns,bodies,contexts):
 expr=f'case {s} {{ {p} -> {b} }}';cases.append(ctx.replace('EXPR',expr)+'\n')
proc=subprocess.run([str(r/'syntax-probe')],input=''.join(json.dumps({'source':s,'old':True})+'\n' for s in cases),capture_output=True,text=True,timeout=60)
failures=[]
for s,line in zip(cases,proc.stdout.splitlines()):
 o=json.loads(line);f=o['format']
 if o['parse']=='[]' and 'out' in f and (not f['idem'] or f['parse_out']!='[]'):failures.append({'source':s,'format':f})
(r/'fuzz-failures.json').write_text(json.dumps(failures,indent=2));print(len(cases),'cases',len(failures),'failures',proc.returncode,proc.stderr[:200])
for f in failures[:12]: print(f['source'].strip())
