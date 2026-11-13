//// Syntax three, stage 11 (docs/design/syntax-3.md §2.6.2): what selection
//// rejects. Read with `I_SYNTAX=3`; `i check` gives the errors of
//// `errors.txt`.
(
	(Io) = i/io
	core = i/core
)

Some(A) = (A)
None = ()
Option(A) = Some(A) | None

f = (Some(0)) { "zero" } | (Some(n)) { n } | (None) { "none" }

show = (n: Int) -> String { core.inspect(n) } | (s: String) -> Int { 7 }

// The branch `(Some(n))` is selected, and it gives an `Int`.
y: String = f(Some(5))

// No branch takes a `Float`.
w = show(1.5)

// Only the branch `(n: Int)` is selected, and it gives a `String`.
u: Int = show(1)

main = () <- (io: Io) {
	io.print(y)
	io.print(core.inspect((w, u)))
}
