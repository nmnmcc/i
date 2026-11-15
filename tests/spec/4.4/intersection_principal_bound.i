// spec: 4.4, 4.7
// expect: pass

// Once Int is a member, no generic component needs to be specialized.
choose = <A, B>(x: A & B & Int, a: A, b: B) { (x + 1, a, b) }
swap = <A, B>(x: B & A & Int, a: A, b: B) { (x + 1, a, b) }
use = () { (choose(1, "s", 1), swap(1, "s", 1)) }

main = () {
	assert(use() == ((2, "s", 1), (2, "s", 1)))
}
