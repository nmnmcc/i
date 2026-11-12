Stream(a, e, n) = (() <- (Fail(e)) -> Step(a, e, n), (n) -> Nil)
Done = ()
Next = (a, Stream(a, e, n))
Step(a, e, n) = Done | Next
new = () -> Stream(a, Never, n) {
	Stream(() -> Step(a, Never, n) { Done }, (_: n) { Nil })
}
Bias = (value = Int)
needs_bias = () -> Stream(Int, Never, Bias) { new() }
