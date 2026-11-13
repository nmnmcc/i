(
	spec/defs
	spec/mid
)

(
	main
)

main = () {
	assert(mid.relay((a = "ok"), "a") == ("ok", 7))
	assert(mid.relay((c = 9), "c") == (9, 7))
	value = defs.set((a = 1), "a", "s")
	assert(defs.get(value, "a") == "s")
	Nil
}
