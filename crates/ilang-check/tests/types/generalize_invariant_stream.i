// §4.5 rules 4 and 6: N goes in and comes out, so it does not change.
// §4.7: simplification must preserve the principal type of this factory.
Stream(A, E, N) {
	Stream(() -> Step(A, E, N) ? E ! N, (N) -> Nil)
}

Step(A, E, N) {
	Done
	Next(A, Stream(A, E, N))
}

new(A, N) = () -> Stream(A, Never, N) {
	Stream(() -> Step(A, Never, N) ! N { Done }, (_: N) { Nil })
}

Bias {
	Bias(value: Int)
}

needs_bias = () -> Stream(Int, Never, Bias) { new() }

// §4.8 rule 5: a stored computation may use Bias when it is called.
make(A, E, N) = (pull: () -> Step(A, E, N) ? E ! N) -> Stream(A, E, N) {
	Stream(pull, (_: N) { Nil })
}

biased = () -> Stream(Int, Never, Bias) {
	make((b ! Bias) { Next(b.value, new()) })
}
