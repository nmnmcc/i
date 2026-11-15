// spec: 4.9, 14
// expect: pass
(
	i/record
)

(
	main
)
Codec(D, E) = (decode = () -> D, encode = (E) -> String)
object : Codec(Record, Record) = Codec(decode = () { () }, encode = (x) { "" })
field = <R, E, K, V>(s : Codec(R, E), key : K, value : Codec(V, V)) -> Codec(
	(k = V),
	E & (k = V),
) {
	Codec(
		decode = () { record.put(s.decode(), key, value.decode()) },
		encode = (x) { s.encode(x) <> value.encode(record.get(x, key)) },
	)
}

main = () {
	value = Codec(decode = () { 1 }, encode = (x : Int) { "x" })
	user = field(object, "age", value)
	assert(user.decode().age == 1)
	assert(user.encode((age = 2)) == "x")
	Nil
}
