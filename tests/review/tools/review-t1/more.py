from probes import *
BASE='/home/agent/Programming/i/.rd/a/target/t1/continued/i-baseline-94789e6'
run('host_equivalent_whole','''Whole = {Int - 0} | 0
@external("app", "read")
read: () -> Whole
''')
run('host_equivalent_finite','''One = {1 | 2} - 2
@external("app", "write")
write: (One) -> Nil
''')
run('host_equivalent_inter','''One = Int & 1
@external("app", "write")
write: (One) -> Nil
''')
run('host_service','''@host("app")
Svc { Svc(get: () -> "a" ? "error") }
''')
run('annotation_removed','''Cell(A) { Cell(get: () -> A, put: (A) -> Nil) }
mk(A) = (x: A) { Cell(() { x }, (_: A) { Nil }) }
f = () { c = mk(0) c.put(1) c.get() }
@main
main = () { assert(f()==0) Nil }
''',True)
run('annotation_added','''Cell(A) { Cell(get: () -> A, put: (A) -> Nil) }
mk(A) = (x: A) { Cell(() { x }, (_: A) { Nil }) }
f = () { c: Cell(Int) = mk(0) c.put(1) c.get() }
@main
main = () { assert(f()==0) Nil }
''',True)
run('typed_function_acc','''(list = std/list)
@main
main = () {
 assert(list.fold([1,2,3],0,(n: Int,x: Int) { { n == 3 } |> { True -> n  False -> n+x } })==3)
 Nil
}
''',True)
for name,src in {
 'regress_const':'x = 1\nf = () { x == 2 }\n',
 'regress_result':'status = () { "ok" }\nf = () { status() == "error" }\n',
 'regress_map':'(list = std/list)\nf = () { list.map(["a"],(x) { x == "b" }) }\n',
 'regress_cells':'Cell(A) { Cell(get: () -> A, put: (A) -> Nil) }\nc = Cell(() { 0 }, (x: Int) { Nil })\nf = () { c.put(1) }\n',
 'regress_single':'f = (s: String) { s |> { "a" -> 1 } }\n'
}.items():
 run(name,src)
 run(name+'-before',src,binary=BASE)
