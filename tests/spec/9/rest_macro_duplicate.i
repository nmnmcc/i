// spec: 9.2, 14
// expect: compile-error E0225
(
	testlib = (test) = i/test
)
Db = (Int)
read = () <- (db: Db) {
	Db(value) = db
	value
}
test("one provide keeps its duplicate-key error through macro expansion", () {
	read() <- (Db(1), Db(2))
})
