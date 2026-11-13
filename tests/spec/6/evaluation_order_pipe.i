// spec: 6.2
// expect: stop "panic: left"
// at: spec/main:10:2

// `a |> f(b)` works out `a` first.
pick = (n: Int) -> (Int, Int) -> Int { panic("callee") }

main = () {
	n = 1
	panic("left") |> pick(n)(panic("argument"))
	Nil
}
