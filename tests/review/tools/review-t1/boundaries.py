from probes import *
run('negative_group_break','''f = (x: -{
1}) { x }
''')
run('ordinary_group_break','''f = (x: {
Int}) { x }
''')
run('display_tuple','x = ('+', '.join(str(i) for i in range(15))+')\nf = () { x == "wrong" }\n')
run('display_union','Many = '+' | '.join(str(i) for i in range(-20,21))+'\nf = (x: Many) { x == "wrong" }\n')
p=make('host_export_input','''(f)
f = (x) { x |> { "a" -> 1  "b" -> 2 } }
''');(p/'i.toml').write_text('name="probe"\n[wasm]\nexports=["probe/main.f"]\n')
r=subprocess.run([BIN,'check','--project',str(p),'--types'],text=True,capture_output=True);(p/'check.txt').write_text(r.stdout+r.stderr);print('export_input',r.returncode,r.stderr)
lib=ROOT/'cross-lib';(lib/'src').mkdir(parents=True,exist_ok=True);(lib/'i.toml').write_text('name="lib"\n')
lits=' | '.join('"v%02d"'%i for i in range(20))
(lib/'src/api.i').write_text('(Modes, last, identity)\nModes = '+lits+'\nlast = "v19"\nidentity = (m: Modes) { m }\n')
p=make('cross-consumer','''(api = lib/api)
f = (x: api.Modes) { api.identity(x) }
@main
main = () { assert(f(api.last) == "v19") Nil }
''');(p/'i.toml').write_text('name="probe"\n[dependencies]\nlib={path="../cross-lib"}\n')
out=[]
for cmd in [['check','--types'],['run','--opt','0'],['run','--opt','2']]:
 r=subprocess.run([BIN,*cmd,'--offline','--project',str(p)],text=True,capture_output=True);out.append(dict(cmd=cmd,exit=r.returncode,stdout=r.stdout,stderr=r.stderr));print('cross-package',cmd,r.returncode,r.stderr)
(p/'result.json').write_text(json.dumps(out,indent=2))
(lib/'src/api.i').write_text((lib/'src/api.i').read_text().replace('"v19"','"v99"'))
r=subprocess.run([BIN,'check','--offline','--types','--project',str(p)],text=True,capture_output=True);(p/'changed.txt').write_text(r.stdout+r.stderr);print('cross-package-changed',r.returncode,r.stderr)
