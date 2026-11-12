//// Syntax three, stage 10 (docs/design/syntax-3.md §2.6.5, §6.4): a
//// multi-shape function is a value. It is kept in a list, taken out of it,
//// called with each of its shapes, passed, returned, compared and shown.
//// Read with `I_SYNTAX=3`; `i run` prints the lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
)

area =
	(r: Float) -> Float { 3.0 *. r *. r }
	| (w: Float, h: Float) -> Float { w *. h }

connect =
	(host: String) -> String { connect(host, port = 80) }
	| (host: String, port = p: Int) -> String { "\(host):\(core.inspect(p))" }

fs = [area]

/// The first item, or `d` for an empty list.
first = (xs, d) {
	xs |> ([]) { d } | ([x, .._]) { x }
}

pick = () { area }

main = () <- (io: Io) {
	f = first(fs, area)
	io.print(core.inspect(f(2.0)))
	io.print(core.inspect(f(3.0, 4.0)))
	g = pick()
	io.print(core.inspect(g(1.0, 2.0)))
	c = first([connect], connect)
	io.print(c("a.com"))
	io.print(c("b.com"))
	io.print(core.inspect(f))
	io.print(core.inspect(c))
	io.print(core.inspect(f == f))
	io.print(core.inspect(f == g))
	io.print(core.inspect(f == c))
}
