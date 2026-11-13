//// Syntax three, stage 9 (docs/design/syntax-3.md §2.6.2 step 2): a direct
//// call with a shape that the chain does not have.
//// `i check` gives the errors of `errors.txt`.
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

main = () <- (io: Io) {
	io.print(
		core.inspect(area(1.0, 2.0, 3.0)),
	)
	io.print(connect("a.com", 80))
	io.print(connect("a.com", prot = 80))
}
