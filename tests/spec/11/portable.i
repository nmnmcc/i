// spec: 11.9, 13
// expect: pass
// multicore.md D3, 3.1, 3.6: walk exact captures, including cycles, without recursion.
(
	i/core
	i/control
	i/key
)

repeat = (n, xs) {
	{ n == 0 } |> (True) { xs } | (False) { repeat(n - 1, [Nil, ..xs]) }
}

main = () {
	assert(core.portable((Nil, True, 42, 3.0, "a", <<1>>, [1, 2])))
	assert(
		core.portable(repeat(1000000, [])),
	)
	control.boundary((b) {
		recursive = (n) {
			{ n == 0 } |> (True) { Nil } | (False) { recursive(n - 1) }
		}
		pinned = (n) { { n == 0 } |> (True) { b } | (False) { pinned(n - 1) } }
		_unused = b
		pure = () { 1 }
		assert(core.portable(pure))
		assert(core.portable(recursive))
		assert(core.portable(recursive))
		assert(
			core.not(core.portable(b)),
		)
		assert(
			core.not(core.portable(pinned)),
		)
		assert(
			core.not(
				core.portable(
					key.wrap(key.new(), b),
				),
			),
		)
		assert(
			core.not(
				core.portable(() {
					{ False } |> (True) { b } | (False) { Nil }
				}),
			),
		)
		Nil
	})
}
