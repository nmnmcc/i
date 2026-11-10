from probe import check
s='S(N) { S(() -> Nil ! N, (N) -> Nil) }\nuse(N) = (s ! S(N)) { S(f, _) = s f() }\n'
check('mutual_finite',s+'''first(N) = (b: Bool, s ! S(N)) {
 b |> { True -> second(False) False -> use() }
}
second(N) = (b: Bool, s ! S(N)) {
 b |> { True -> first(False) False -> use() }
}''')
check('mutual_recursive',s+'''first(N) = (s ! S(N)) -> Nil ! N { second() }
second(N) = (s ! S(N)) -> Nil ! N { first() use() }''')
check('local_env_closure',s+'''Dep { Dep(value: Int) }
get = (d ! Dep) { d.value }
make = (n: Int) {
 () {
   ! Dep = Dep(n)
   ! S(Dep) = S(() { get() Nil }, (_: Dep) { Nil })
   use()
 }
}
example = () { f = make(7) f() }''')
check('local_recursive_closure',s+'''make(N) = () {
 () {
   ! S(N) = S(() { use() }, (_: N) { Nil })
   use()
 }
}''')
check('captured_rows',s+'''capture = (body) { local = () { body() } local }
example = () {
 ! S(Never) = S(() { Nil }, (_: Never) { Nil })
 capture(() { use() })()
}''')
base='''Cell(A) { Cell(read: () -> A, write: (A) -> Nil) }
int = (c ! Cell(Int)) { c.read() }
text = (c ! Cell(String)) { c.read() }
apply = (f) { f() }
'''
check('hidden_conflict_closure',base+'''both = (a, b) { () { a() b() } }
bad = () { both(() { int() }, () { text() })() }''')
check('hidden_conflict_nested',base+'''both = (a, b) { () { a() b() } }
bad = () { both(() { int() }, () { text() }) }''')
check('hidden_conflict_install',base+'''both = (a, b) { () { a() b() } }
bad = () {
 ! Cell(Int) = Cell(read: () { 1 }, write: (_: Int) { Nil })
 both(() { int() }, () { text() })()
}''')
check('nested_row_recur',s+'''bad(N) = (s ! S(N)) -> Nil ! S(N) { S(f, _) = s f() }''')
check('recursive_covariant','''Box(A) { Box(A) }
bad = (x) { bad(Box(x)) }''')
check('recursive_invariant','''Cell(A) { Cell(() -> A, (A) -> Nil) }
bad = (x) { bad(Cell(() { x }, (_) { Nil })) }''')
check('recursive_annotated','''bad(A) = (x: A) -> A { bad([x]) }''')
check('recursive_never','''bad = (x: Never) { x(x) }''')
check('finite_never','''bad = (x: Never) { bad([x]) }''')
