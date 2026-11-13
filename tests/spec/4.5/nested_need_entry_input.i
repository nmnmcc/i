// spec: 4.4, 4.5
// expect: compile-error E0501

// T1: putting a need inside a row-parameterized type keeps its direction.
Read(A) = (read = () -> A)
RowJob(N) = (run = () <- N -> Int)
Job(A) = (job = RowJob(Read(A)))
up = (x: Job(Int)) -> Job(Int | String) { x }
main = () {
	j = up(Job(job = RowJob(run = () <- (r: Read(Int)) { r.read() + 1 })))
	n = j.job.run() <- (Read(read = () { "s" }))
	assert(n == 1)
}
