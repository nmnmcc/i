// spec: 4.4, 4.5
// expect: compile-error E0501

// T1: the String provider must not reach the integer addition.
Read(A) = (read = () -> A)
Job(A) = (run = () <- (Read(A)) -> Int)
up = (x: Job(Int)) -> Job(Int | String) { x }
main = () {
	j = up(Job(run = () <- (r: Read(Int)) { r.read() + 1 }))
	n = j.run() <- (Read(read = () { "s" }))
	assert(n == 1)
}
