Wrap = (f = (Int) -> Wrap)

Two = (f = (Int) -> Two, n = Int)

// `a |> C` is `C(a)`, and `a |> C(b)` is `C(a, b)` (§6.9), so they are
// direct values when their parts are (§6.10).
wrapped = ((n) { wrapped }) |> Wrap

direct = Wrap(f = (n) { direct })

two = ((n) { two }) |> Two(f = 1)

local = () {
	w = ((n) { w }) |> Wrap
	t = ((n) { t }) |> Two(f = 1)
	(w, t)
}

// Not direct: a function call, or a part that is not direct.
id = (x) { x }

not_ctor = ((n) { not_ctor }) |> id

not_direct_arg = ((n) { not_direct_arg }) |> Two(f = id(1))

local_bad = () {
	v = ((n) { v }) |> id
	v
}
