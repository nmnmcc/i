(
	spec/defs
)

(
	relay
)
relay = <K, V>(r : (k = V), key : K) -> (V, Int) {
	inner = <K>(s : (k = Int), k : K) -> Int { defs.get(defs.set(s, k, 7), k) }
	(defs.get(r, key), inner((b = 2), "b"))
}
