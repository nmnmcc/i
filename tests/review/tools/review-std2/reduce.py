from pathlib import Path
import subprocess
root=Path('/tmp/review-std2')
cases={
 'marker':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\nexample = () { ! S(Never) = S(() { Nil }, (_) { Nil }) use() }\n',
 'error':'(i/fail)\nS(E,N) { S(() -> Nil ? E ! N) }\nuse(E,N) = (s ! S(E,N)) { S(f) = s f() }\nexample = () { ! S(String,Never) = S(() { fail.raise("x") }) fail.catch(use) }\n',
 'marker_pure':'S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\nexample = () { ! S(Never) = S(() { Nil }, (_: Never) { Nil }) use() }\n',
 'replacement_unused':'S(E) { S(() -> Nil ? E) }\nexample = () { ! S(String) = S(() { Nil }) ! S(Int) = S(() { Nil }) Nil }\n',
 'replacement_scope':'(i/fail)\nS(E) { S(() -> Nil ? E) }\nuse(E) = (s ! S(E)) { S(f) = s f() }\nfirst: () -> Nil ? String ! S(String) = use\nsecond: () -> Nil ? Int ! S(Int) = use\n@main\nexample = () {\n { ! S(String) = S(() { fail.raise("s") }) Error("s") = fail.catch(first) Nil }\n { ! S(Int) = S(() { fail.raise(2) }) Error(2) = fail.catch(second) Nil }\n}\n',
}
for name,src in cases.items():
 p=root/name;(p/'src').mkdir(parents=True,exist_ok=True);(p/'i.toml').write_text('name = "probe"\n');(p/'src/main.i').write_text(src)
 r=subprocess.run(['/home/agent/Programming/i/.rd/cache/bin/i-1df2fe0','check','--project',str(p),'--types'],capture_output=True,text=True)
 print(name,r.returncode,r.stdout+r.stderr)
 (root/(name+'.out')).write_text(r.stdout+r.stderr)
