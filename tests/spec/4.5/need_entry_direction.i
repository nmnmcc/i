// spec: 4.4, 4.5
// expect: pass
Read(A) = (read = () -> A)
Write(A) = (write = (A) -> Nil)
Reader(A) = (run = () <- (Read(A)) -> Int)
Writer(A) = (run = () <- (Write(A)) -> Nil)
Paused(A) = (k = Continuation(Nil, Int, Read(A)))
RowJob(N) = (run = () <- N -> Int)
Nested(A) = (job = RowJob(Read(A)))
reader = (x: Reader(Int | String)) -> Reader(Int) { x }
writer = (x: Writer(Int)) -> Writer(Int | String) { x }
paused = (x: Paused(Int | String)) -> Paused(Int) { x }
nested = (x: Nested(Int | String)) -> Nested(Int) { x }

main = () {
	r = Reader(run = () <- (s: Read(Int | String)) { 7 })
	w = Writer(run = () <- (s: Write(Int)) { s.write(1) })
	assert({ reader(r).run() <- (Read(read = () { 1 })) } == 7)
	writer(w).run() <- (Write(write = (_: Int | String) { Nil }))
}
