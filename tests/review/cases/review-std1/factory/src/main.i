Stream(a, e, n) {
 Stream(() -> Step(a, e, n) ? e ! n, (n) -> Nil)
}
Step(a, e, n) { Done Next(a, Stream(a, e, n)) }
new = () -> Stream(a, Never, n) {
 Stream(() -> Step(a, Never, n) ! n { Done }, (_: n) { Nil })
}
Bias { Bias(value: Int) }
needs_bias = () -> Stream(Int, Never, Bias) { new() }
