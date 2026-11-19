(
	core = (List, String) = i/core
	other = spec/other
)
Int = (core.Int)
unwrap = (x : Int) -> core.Int { x |> (Int(n)) { n } }
items = (xs : core.List(core.Int)) -> List(core.Int) { xs }
identity = (x : core.Int) -> core.Int { other.identity(x) }
label = (x : String) -> core.String { x }
result = (x : Result(core.Int, ())) -> Result(core.Int, ()) {
	x
}
main = () {
	assert(
		identity(unwrap(Int(42))) == 42,
	)
}
