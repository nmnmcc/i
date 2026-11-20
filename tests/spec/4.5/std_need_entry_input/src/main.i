(
	i/stream
	i/log
)

Read(A) = (read = () -> A)
Pull(A) = (stream = stream.Stream(Int, Never, Read(A)))
Log(A) = (sink = log.Sink(Never, Read(A)))
pull = (x : Pull(Int)) -> Pull(Int | String) { x }
send = (x : Log(Int)) -> Log(Int | String) { x }
main = () { () }
