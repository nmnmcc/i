// spec: 4.8, 9.2, 9.4, 14
// expect: pass
(
	testlib = (test) = std/test
)
Db = (value = Int)
read = () <- (db: Db) { db.value }
test("captured rest rows survive macro expansion", () {
	save = <N>(body: () <- N -> Int) <- (..env: N) {
		() {
			value = body() <- env
			value
		}
	}
	kept = save(read) <- (Db(value = 42))
	assert({ kept() <- (Db(value = 0)) } == 42)
	assert({ read() <- (Db(value = 2)) <- (Db(value = 1)) } == 2)
})
