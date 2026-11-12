//// Syntax three, stage 9 (docs/design/syntax-3.md §2.6.5, §4.5): the type
//// of a multi-shape value. Read with `I_SYNTAX=3`; `i check --types` prints
//// the lines of `expected.txt`.
(
	(Io) = i/io
)

area =
	(r: Float) -> Float { 3.0 *. r *. r }
	| (w: Float, h: Float) -> Float { w *. h }

fs = [area]

both = [area, area]

main = () <- (io: Io) {
	gs = fs
	io.print("made")
}
