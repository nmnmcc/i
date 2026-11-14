//// Syntax three, stage 9 (docs/design/syntax-3.md §2.6.5, §4.5): the type
//// of a multi-shape value. `i check --types` prints
//// the lines of `types.txt`.
(
	(Io) = i/io
)

area =
	(r : Float) -> Float { 3.0 *. r *. r }
	| (w : Float, h : Float) -> Float { w *. h }

fs = [area]

/// The type as it is written (§2.4).
typed : List({ (Float) | (Float, Float) } -> Float) = [area]

/// One shape with a label: the label is part of the type.
scale = (x : Int, by = k : Int) -> Int { x * k }

main = () <- (io : Io) {
	_ = (fs, typed, scale(2, by = 3))
	io.print("made")
}
