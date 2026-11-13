(
	i/record
)

(
	set
	get
)
set = <R, K, V>(r: R & Record, key: K, value: V) -> (k = V) {
	record.put(r, key, value)
}
get = <K, V>(r: (k = V), key: K) -> V { record.get(r, key) }
