(
	app/deep/third
)

(
	mk
	T(..)
	Shown(..)
	far
)

T {
	T(Int)
}

Shown {
	Shown
}

mk = (n: Int) { T(n) }

far = () { third.make() }
