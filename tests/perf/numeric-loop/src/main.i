(
	i/stdio
	i/core
	kernels = numeric/kernels
)

n = 1_000_000

loop = (k : (Float) -> Float, i : Int, acc : Float) -> Float {
	i |> (0) { acc } | (_) { loop(k, i - 1, 0.03 *. k(acc)) }
}

run = (k : (Float) -> Float) {
	stdio.print(
		core.inspect(loop(k, n, 0.5)),
	)
}

base = () { run((x) { x }) }
identity = () { run(kernels.identity) }
fast = () { run(kernels.fast) }
horner = () { run(kernels.horner) }
integer = () { run(kernels.integer) }
dot = () { run(kernels.dot) }
tuples = () { run(kernels.tuples) }
closures = () { run(kernels.closures) }
