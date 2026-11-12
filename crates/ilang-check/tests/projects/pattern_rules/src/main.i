(a, a) = (1, 2)

f = () {
	[x, ..x] = [1]
	y = (z, z) = (1, 2)
	"a" <> w = "ab"
	((c, b) = b) = (1, 2)
	Nil
}

params = (p, p) { p }

labels = (_ = x = l, _ = y = l) { x }

env = (s) <- (s: Assert) { s }

typed = (n: Int, _ = n = m) { n }

choices = (v) {
	v |>
		((a, b) | (a, _)) { a }
		| (Ok(x) | Error(y)) { x }
		| (Ok(x) | Error(x)) { x }
		| ([k, ..] | [_, k, ..] | []) { 0 }
}

ctor = (v) { v |> (Nope(x)) { x } | (Ok(n)) { n } }
