// spec: 2.5, 11.7
// expect: stop "panic: not ready"
// at: spec/stop_in_start:15:12
// at: spec/stop_in_start:10:9

(
	i/core
)

ready = check(1)

check = (n: Int) {
	{ n > 5 } |> {
		True  -> n
		False -> panic("not ready")
	}
}

// No entry point runs when working out a value stops.

main = () { core.inspect(ready) }
