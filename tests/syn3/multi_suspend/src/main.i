//// Syntax three, stage 10 (docs/design/syntax-3.md §6.4): a call through a
//// multi-shape value across `control.suspend`, and multi-shape values given
//// to the control operations as the body and the handler.
//// `i run` prints the lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
	control = i/control
)

/// The one-position group suspends to the boundary `b`; the handler goes on
/// with 10.
ask =
	(b) -> Int { control.suspend(b, (k) { control.resume(k, 10) }) + 1 }
	| (b, extra : Int) -> Int { extra }

/// A multi-shape value as the body of a boundary.
body = (b) -> Int { ask(b) * 2 } | (b, x : Int) -> Int { x }

run = (f, b) -> Int { f(b) * 3 }

main = () <- (io : Io) {
	io.print(
		core.inspect(control.boundary((b) { run(ask, b) })),
	)
	io.print(
		core.inspect(control.boundary(body)),
	)
}
