// typed macro opaque graph handle in expression position
(
	m = spec/macros
)
f = (x: { (a = Int) | (b = String) }) { m.m(x) }
main = () { f((a = 42)) }
