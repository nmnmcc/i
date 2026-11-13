//// Syntax three, stages 9 and 10 (docs/design/syntax-3.md §2.6.5, §6.4): a
//// multi-shape value called through a value, and a call with labels through
//// a value. Read with `I_SYNTAX=3`; `i run` prints the lines of
//// `expected.txt`.
(
	(Io) = i/io
	core = i/core
)

area =
	(r: Float) -> Float { 3.0 *. r *. r }
	| (w: Float, h: Float) -> Float { w *. h }

first = <A>(xs: List(A), d: A) -> A { xs |> ([]) { d } | ([x, ..]) { x } }

/// A call with labels through a value.
via = (c) { c("b.com", port = 8080) }

main = () <- (io: Io) {
	f = first([area], area)
	io.print(core.inspect(f(2.0)))
	io.print(via((host: String, port = p: Int) { host }))
}
