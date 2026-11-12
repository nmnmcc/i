// spec: 9.7, 9.3, 11.7
// expect: stop "error: "no missing""
// at: spec/main:14:1

// An error of a host function that comes to the program entry stops it.
@host("test:kv/v1")
Kv {
	Kv(get: (String) -> String ^ String)
}

lookup = (key: String, kv ! Kv) { kv.get(key) }


main = () {
	lookup("missing")
	Nil
}
