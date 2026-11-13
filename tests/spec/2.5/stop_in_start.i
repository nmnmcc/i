// spec: 2.5, 11.7
// expect: stop "panic: not ready"
// at: spec/main:11:58
// at: spec/main:9:9
(
	i/core
)

ready = check(1)

check = (n: Int) { { n > 5 } |> (True) { n } | (False) { panic("not ready") } }

// No entry point runs when working out a value stops.
main = () { core.inspect(ready) }
