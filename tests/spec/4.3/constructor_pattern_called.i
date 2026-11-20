// spec: 4.3, 7.1
// expect: pass
// A constructor is a function: `True` is `() -> True`. A pattern matches
// its value only as the call `True()`, which this program writes.
(
	main
)
r = (x : Bool) -> Int { x |> (True()) { 1 } & (False()) { 0 } }

main = () {
	assert(r(True) == 1)
	()
}
