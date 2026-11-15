(
	app/deep/third
)

(
	mk
	T(..)
	Shown(..)
	far
)

T = (Int)

Shown = ()

mk = (n : Int) { T(n) }

far = () { third.make() }
