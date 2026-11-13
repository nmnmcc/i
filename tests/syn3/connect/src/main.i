//// Syntax three, stage 9 (docs/design/syntax-3.md §2.6, §4.5): a chain with
//// several shape groups, and direct calls that choose the group by the
//// shape of their argument data. `i run` prints the
//// lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
)

Conn = (host = String, port = Int)

open = (host: String, port: Int) -> Conn { Conn(host = host, port = port) }

connect =
	(host: String) -> Conn { connect(host, port = 80) }
	| (host: String, port = p: Int) -> Conn { open(host, p) }

area =
	(r: Float) -> Float { 3.0 *. r *. r }
	| (w: Float, h: Float) -> Float { w *. h }

/// One shape with a label: the label is part of the shape.
scale = (x: Int, by = k: Int) -> Int { x * k }

/// Groups are kept apart by shape; inside a group the first match wins.
describe =
	(0) { "zero" }
	| (a: Int, b: Int) { "pair \(core.inspect(a + b))" }
	| (n: Int) { "one \(core.inspect(n))" }
	| () { "none" }

show = (c: Conn) -> String { "\(c.host):\(core.inspect(c.port))" }

main = () <- (io: Io) {
	io.print(
		show(connect("a.com")),
	)
	io.print(
		show(connect("b.com", port = 8080)),
	)
	io.print(
		core.inspect(area(2.0)),
	)
	io.print(
		core.inspect(area(3.0, 4.0)),
	)
	io.print(
		core.inspect(scale(6, by = 7)),
	)
	io.print(describe(0))
	io.print(describe(5))
	io.print(describe(2, 3))
	io.print(describe())
	// A pipe is a direct call too: `x` is position 0.
	io.print(core.inspect(2.0 |> area))
	io.print(
		core.inspect(3.0 |> area(5.0)),
	)
	// A direct alias keeps the groups.
	c = connect
	io.print(
		show(c("c.com", port = 1)),
	)
	// A local chain.
	twice = (x: Int) { x * 2 } | (x: Int, y: Int) { { x + y } * 2 }
	io.print(
		core.inspect(twice(4) + twice(1, 2)),
	)
}
