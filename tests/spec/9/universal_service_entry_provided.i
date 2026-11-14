// spec: 9.2
// expect: pass
// A real provider satisfies Sink(Never); its get method remains callable.
Sink(A) = (put = (A) -> Nil, get = () -> Int)
read = () <- (s : Sink(Never)) { s.get() }
main = () {
	assert(read() == 42) <- (Sink(put = (_) { Nil }, get = () { 42 }))
}
