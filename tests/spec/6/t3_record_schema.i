// spec: 4.9, 14
// expect: pass
(
	i/record
)

(
	main
)
Schema(A) = (decode = () -> A)
object = Schema(decode = () { () })
field = <R, K, V>(s: Schema(R), key: K, value: Schema(V)) -> Schema((k = V)) {
	Schema(decode = () { record.put(s.decode(), key, value.decode()) })
}

main = () {
	name = Schema(decode = () { "Ada" })
	age = Schema(decode = () { 36 })
	user = field(field(object, "name", name), "age", age)
	result = user.decode()
	assert(result.name == "Ada")
	assert(result.age == 36)
	Nil
}
