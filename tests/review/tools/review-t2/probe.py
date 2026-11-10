from pathlib import Path
import subprocess,json,time
ROOT=Path('/tmp/review-t2')
BIN='/home/agent/Programming/i/.rd/t2/target/release/i'
def case(name,src,expect='pass',files=None):
 p=ROOT/name;(p/'src').mkdir(parents=True,exist_ok=True)
 (p/'i.toml').write_text('name = "review"\n')
 (p/'src/main.i').write_text(src+'\n')
 for path,text in (files or {}).items():
  f=p/path;f.parent.mkdir(parents=True,exist_ok=True);f.write_text(text+'\n')
 t=time.monotonic()
 try:
  r=subprocess.run([BIN,'check','--project',str(p),'--types'],capture_output=True,text=True,timeout=12)
  out=r.stdout+r.stderr; status=r.returncode
 except subprocess.TimeoutExpired: out='TIMEOUT';status=124
 (p/'check.txt').write_text(out)
 result={'case':name,'expected':expect,'exit':status,'seconds':round(time.monotonic()-t,3),'output':out}
 print(json.dumps(result),flush=True)
 return result
if __name__=='__main__':
 cases={
 'cycle_alias':('A = f\nf = (x: A) { x }','E0530'),
 'cycle_mixed':('a = (x: b) { x }\nb = a(1)','E0530'),
 'mutual_ok':('even = (n: Int) { n == 0 || odd(n - 1) }\nodd = (n: Int) { n != 0 && even(n - 1) }','pass'),
 'mutual_type':('even = (n: odd) { odd(n) }\nodd = (n) { even(n) }','E0530'),
 'shadow':('x: String = "s"\nf = (x: Int) { y: x = 1  y }\ng = () { x: Bool = True  y: x = False  y }','pass'),
 'shadow_reserved':('f = () { g = (f: Int) { x: f = 1  x }  g(2) }','pass'),
 'pending':('x: Int = 1\nf = (x, y: x) { y }','E0536'),
 'generalized_alias':('id = (x) { x }\nId = id','E0533'),
 'generalized_anno':('id = (x) { x }\na: id = (x: Int) { x + 1 }\nb: id = (x: String) { x <> "!" }','pass'),
 'weak_alias':('id = (x) { x }\nv = id(1)\nRead = v\nx: Read = 2','pass'),
 'weak_anno':('id = (x) { x }\nv = id(1)\nx: v = 2','inspect'),
 'unfilled':('id = (x) { x }\nv = id(id)\nRead = v','E0534'),
 'mismatch':('Box(A) { Box(A) }\nv: Int = 1\nBox(_) = v','E0535'),
 'extra_union':('Box(A) { Box(A) }\nv: Box(Int) | Nil = Box(1)\nBox(Item) = v','E0535'),
 'duplicate':('Pair(A,B) = (A,B)\nv = (1,2)\nPair(Item,Item) = v','E0208'),
 'partial':('Box(A,B) { Box(A,B) }\nv = Box(1,(x) {x})\nBox(Item,_) = v\nx: Item = 2','pass'),
 'shared_skip':('Box(A,B) { Box(A,B) }\nmake(A) = (x:A) {Box(x,x)}\nv = make(1)\nBox(Item,_) = v\nx: Item = 2','pass'),
 'nested':('Pair(A,B) = (A,B)\nBox(A) { Box(A) }\nv = (Box(1),"s")\nPair(Box(Item),_) = v\nx: Item = 2','pass'),
 'exact_good':('Pair(A) = (A,Int)\nv = ("s",1)\nPair(Item) = v','pass'),
 'exact_bad':('Pair(A) = (A,Int)\nv = ("s","s")\nPair(Item) = v','E0535'),
 'repeated_formal':('Pair(A) = (A,A)\nv = (1,1)\nPair(Item) = v','E0532'),
 'union_samehead':('Box(A) { Box(A) }\nchoose = (b: Bool) { b |> { True -> Box(1)  False -> Box("s") } }\nv = choose(True)\nBox(Item) = v\nx: Item = "s"','pass'),
 'union_samehead_direct':('Box(A) { Box(A) }\nv = True |> { True -> Box(1)  False -> Box("s") }\nBox(Item) = v','pass'),
 'recovery':('Box(A) { Box(A) }\nv: Int = 1\nBox(Item) = v\nbad: Int = "s"','E0535 and E0501'),
 'privacy':('(Read)\nHidden { Hidden }\nv = Hidden\nRead = v','E0227'),
 'schema':('Schema(D,E) { Schema(decode: (String)->D, encode: (E)->String) }\nmake(D,E) = (d:(String)->D,e:(E)->String) {Schema(d,e)}\nuser = make((s:String)->(String,Int) {(s,1)},(v:(String,Int))->String {v.0})\nSchema(User,_) = user\nuse = (u: User) {u.0}\nresult = use(("a",2))','pass'),
 }
 for n,(s,e) in cases.items():case(n,s,e)
 case('foreign_closed','((Box) = review/lib)\nBox(Item) = lib.v\nx: Item = 2','pass',{'src/lib.i':'(Box,v)\nBox(A) { Box(A) }\nv: Box(Int) = Box(1)'})
 case('foreign_open','((Box) = review/lib)\nBox(Item,_) = lib.v','E0534',{'src/lib.i':'(Box,v,Local)\nBox(A,B) { Box(A,B) }\nv = Box(1,(x){x})\nBox(Local,_) = v'})
 case('foreign_alias','((Local) = review/lib)\nx: Local = 2','pass',{'src/lib.i':'(Local)\nBox(A,B) { Box(A,B) }\nv = Box(1,(x){x})\nBox(Local,_) = v'})
 case('qualified_alias_pattern','(review/lib)\nBox(A) {Box(A)}\nv = Box(1)\nBox(lib.Scalar) = v','pass',{'src/lib.i':'(Scalar)\nScalar = Int'})
