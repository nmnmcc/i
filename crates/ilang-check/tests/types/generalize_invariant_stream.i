// §4.5 rules 4 and 6: n goes in and comes out, so it does not change.
// §4.7: simplification must preserve the principal type of this factory.
Stream(a, e, n) {
	Stream(() -> Step(a, e, n) ? e ! n, (n) -> Nil)
}

Step(a, e, n) {
	Done
	Next(a, Stream(a, e, n))
}

new = () -> Stream(a, Never, n) {
	Stream(() -> Step(a, Never, n) ! n { Done }, (_: n) { Nil })
}

Bias {
	Bias(value: Int)
}

needs_bias = () -> Stream(Int, Never, Bias) { new() }

// §4.8 rule 5: a stored computation may use Bias when it is called.
make = (pull: () -> Step(a, e, n) ? e ! n) -> Stream(a, e, n) {
	Stream(pull, (_: n) { Nil })
}

biased = () -> Stream(Int, Never, Bias) {
	make((b ! Bias) { Next(b.value, new()) })
}
