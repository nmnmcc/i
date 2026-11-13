// spec: 4.7, 9.8
// expect: pass
// multicore.md D13: one key can collect Int | String, but reads keep that union.
(
	i/key
)

main = () {
	k = key.new()
	number = key.wrap(k, 1)
	text = key.wrap(k, "x")
	assert(key.unwrap(k, number) == key.Found(1))
	assert(key.unwrap(k, text) == key.Found("x"))
	k2 = key.new()
	assert(
		key.unwrap(k2, key.wrap(k2, 1)) == key.Found(1),
	)
}
