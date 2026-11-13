//// Syntax three, stage 9 (docs/design/syntax-3.md §2.6.5): the temporary
//// error of stage 9 for a multi-shape value that is called through a value.
//// Stage 10 removes that error; this fixture then becomes a running one.
//// Read with `I_SYNTAX=3`; `i check` gives the errors of `errors.txt`.
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
	io.print(
		core.inspect(f(2.0)),
	)
	io.print(via((host: String, port = p: Int) { host }))
}
