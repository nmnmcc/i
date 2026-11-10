(i/record)
(set, get)
set(R, K, V) = (r: R & Record, key: K, value: V) -> (..R, K: V) { record.put(r, key, value) }
get(K, V) = (r: (K: V), key: K) -> V { record.get(r, key) }
