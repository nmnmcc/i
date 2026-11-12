(
 core = i/core
 (List, String) = i/core
 prelude = i/prelude
 other = spec/other
)
Int { Int(core.Int) }
unwrap = (x: Int) -> core.Int { x |> { Int(n) -> n } }
items = (xs: core.List(core.Int)) -> List(core.Int) { xs }
identity = (x: core.Int) -> core.Int { other.identity(x) }
label = (x: String) -> core.String { x }
result = (x: prelude.Result(core.Int, prelude.Nil)) -> Result(core.Int, Nil) { x }
main = () { assert(identity(unwrap(Int(42))) == 42) }
