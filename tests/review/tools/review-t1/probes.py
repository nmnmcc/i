from pathlib import Path
import subprocess,json,time
ROOT=Path('/tmp/review-t1'); BIN='/home/agent/Programming/i/.rd/a/target/release/i'
STD='/home/agent/Programming/i/.rd/a/std'
def make(name,src):
 p=ROOT/name;(p/'src').mkdir(parents=True,exist_ok=True)
 (p/'i.toml').write_text(f'name = "probe"\n[dependencies]\nstd = {{ path = "{STD}" }}\n')
 (p/'src/main.i').write_text(src);return p

def run(name,src,execute=False,binary=BIN):
 p=make(name,src); out=[]
 cmds=[['check','--types']]+([['run','--opt','0'],['run','--opt','2']] if execute else [])
 for cmd in cmds:
  t=time.monotonic()
  try:
   r=subprocess.run([binary,*cmd,'--offline','--project',str(p)],capture_output=True,text=True,timeout=40)
   row={'cmd':cmd,'exit':r.returncode,'seconds':time.monotonic()-t,'stdout':r.stdout,'stderr':r.stderr}
  except subprocess.TimeoutExpired: row={'cmd':cmd,'exit':'timeout','seconds':time.monotonic()-t}
  out.append(row)
  if cmd[0]=='check' and row['exit']!=0:break
 (p/'result.json').write_text(json.dumps(out,indent=2))
 print(name,[(r['cmd'],r['exit'],round(r['seconds'],3)) for r in out],flush=True)
 if out[0]['exit']!=0: print(out[0].get('stderr','')[:2500],flush=True)
 return out
if __name__=='__main__':
 run('operators','''a = 1 + 2
b = "a" <> "b"
c = -{1}
d = -{-1}
x = -0
y: 16 = 0x10
s: "é" = "\\u{e9}"
@main
main = () { assert(a == 3) assert(b == "ab") Nil }
''',True)
 run('folds','''(list = std/list)
Box(A) { Box(value: A) }
rec = (n, acc) { n |> { 0 -> acc  _ -> rec(n - 1, acc + 1) } }
@main
main = () {
 assert(rec(4, 0) == 4)
 assert(list.fold([1,2,3], 0, (n,x) { { n == 1 } |> { True -> n + x  False -> n + 1 } }) == 4)
 assert(list.fold([1,2], (0,""), (p,x) { (p.0+x,p.1<>"a") }).1 == "aa")
 assert(list.fold([1,2], Box(0), (p,x) { Box(p.value+x) }).value == 3)
 Nil
}
''',True)
 run('invariant','''Cell(A) { Cell(run: (A) -> A) }
make = () { Cell((x) { x }) }
ci: Cell(Int) = make()
cs: Cell(String) = make()
id(A) = (x: A) -> A { x }
@main
main = () { assert(ci.run(3) == 3) assert(cs.run("b") == "b") assert(id(1) == 1) Nil }
''',True)
 run('invariant_bad','''Cell(A) { Cell(run: (A) -> A) }
bad = (x: Cell(1)) -> Cell(Int) { x }
''')
 run('finite','''f = { 0 -> "zero"  1 -> "one" }
g = (s: "a" | "ab") { s |> { "a" -> 0  "a" <> _ -> 1 } }
@main
main = () { assert(f(1)=="one") assert(g("ab")==1) Nil }
''',True)
 run('missing_int','''f = (n: Int) { n |> { 0 -> 0  1 -> 1 } }
''')
 run('missing_string','''f = (n: String) { n |> { "" -> 0  "a" <> _ -> 1 } }
''')
 run('guards','''f = (n: 0 | 1) { n |> { 0 ? True -> 0  1 -> 1 } }
''')
 run('list_inferred','''f = (xs) { xs |> { [] -> 0  ["a",..] -> 1  ["b",..] -> 2 } }
''')
 run('list_annotated','''f = (xs: List("a" | "b")) { xs |> { [] -> 0  ["a",..] -> 1  ["b",..] -> 2 } }
@main
main = () { assert(f(["b","a"])==2) Nil }
''',True)
 run('nested_reject','''g = (s: String - "a") { s |> { "b" -> 1  _: String - "a" - "b" -> 2 } }
f = (p: (String,Bool)) { p |> {
 ("a",True) -> 0
 (s,_) -> g(s)
} }
''')
 run('cofinite','''g = (s: String) { s |> {
 _: String - "a" - "b" -> 3
 "a" -> 1
 r -> { x: "b" = r  2 }
} }
h = (n: Int | String) { n |> {
 _: String - "a" -> 1
 "a" -> 2
 _: Int - 0 -> 3
 0 -> 4
} }
@main
main = () {
 assert(g("b")==2) assert(g("z")==3)
 assert(h(0)==4) assert(h(1073741824)==3)
 assert(h(-1073741825)==3) assert(h(1000000000000000000000000000000)==3)
 assert(h("b")==1) Nil
}
''',True)
 run('prefix_tuple','''f = (p: (String,Int)) { p |> {
 (_: String - "ab",_) -> 1
 ("a" <> _,n) -> n + 1
 _ -> 0
} }
@main
main = () { assert(f(("ab",5))==6) Nil }
''',True)
 run('assert_group','''(assertion = i/assertion)
pair = ("b", () { ("a",_) = pair  1 })
@main
main = () { assertion.catch(pair.1) |> { Error(_) -> Nil  Ok(_) -> panic("missed Assert") } }
''',True)
 run('error_rows','''(fail = i/fail)
f = (b: Bool) { b |> { True -> fail.raise("a")  False -> fail.raise("b") } }
g = (b: Bool) { fail.catch(() { f(b) }) |> { Error("a") -> 1  Error(e) -> { x: "b" = e  2 }  Ok(_) -> 0 } }
@main
main = () { assert(g(False)==2) Nil }
''',True)
 run('service','''State(A) { State(get: () -> A) }
cur(A) = (seed: A, st ! State(A)) { st.get() }
@main
main = () { ! State(Int) = State(() { 5 }) assert(cur(0)==5) Nil }
''',True)
 run('service_bad','''State(A) { State(get: () -> A) }
f = (s ! State(1)) { s.get() }
g = () { ! State(Int) = State(() { 2 }) f() }
''')
 run('host_return','''Mode = "on" | "off"
@external("app", "mode")
mode: () -> Mode
''')
 run('host_outbound','''Level = "info" | "warn"
@external("app", "log")
log: (Level | "error") -> Nil
@external("app", "number")
number: (Int - 0) -> Nil
''')
 run('equality','''x = "a"
f = () { x == "b" }
g = () { 1 != 2 }
h = (x: Int - 1) { x == 1 }
''')
 run('functions_acc','''(list = std/list)
Box(A) { Box(get: () -> A) }
@main
main = () {
 b = list.fold([1,2,3], Box(() { 0 }), (acc,x) { Box(() { acc.get()+x }) })
 assert(b.get()==6) Nil
}
''',True)
