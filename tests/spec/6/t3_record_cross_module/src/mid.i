(spec/defs)
(relay)
relay(K, V) = (r: (K: V), key: K) -> (V, Int) {
  inner(K) = (s: (K: Int), k: K) -> Int { defs.get(defs.set(s, k, 7), k) }
  (defs.get(r, key), inner((b: 2), "b"))
}
