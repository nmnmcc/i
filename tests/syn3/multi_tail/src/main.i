//// Syntax three, stage 10 (docs/design/syntax-3.md §6.4, spec §11.6):
//// 10^6 tail calls through a multi-shape value, in constant stack. The
//// value is a parameter, so each call goes through the run-time dispatch.
//// Read with `I_SYNTAX=3`; `i run` prints the lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
)

loop =
	(n: Int) -> Int { n }
	| (n: Int, acc: Int) -> Int { n |> (0) { acc } | (_) { again(loop, n - 1, acc + 2) } }

/// Calls the value `f` in tail position.
again = (f, n: Int, acc: Int) -> Int { f(n, acc) }

main = () <- (io: Io) {
	io.print(core.inspect(again(loop, 1_000_000, 0)))
	io.print(core.inspect(loop(5)))
}
