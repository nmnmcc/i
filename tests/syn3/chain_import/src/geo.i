//// The chains that `main` calls.
(
	core = i/core
)

(
	area
	show
)

area =
	(r : Float) -> Float { 3.0 *. r *. r }
	| (w : Float, h : Float) -> Float { w *. h }

show = (n : Int) -> String { core.inspect(n) } | (s : String) -> Int { 7 }
