//// Typed keys for heterogeneous immutable values (§9.8).
(
	new
	wrap
	unwrap
	Lookup(..)
)

Found(A) = (A)

Missing = ()
Lookup(A) = Found(A) | Missing
/// Makes a fresh identity key. A key's answer type is invariant.
@external("ilang:key", "new")
new : <A>() -> Key(A)

/// Marks a value with its key.
@external("ilang:key", "wrap")
wrap : <A>(Key(A), A) -> Keyed

@external("ilang:key", "read")
read : <A>(Key(A), Keyed) -> Result(A, Nil)

/// Reads only a value marked by this same key.
unwrap = <A>(k : Key(A), x : Keyed) -> Lookup(A) {
	read(k, x) |> (Ok(v)) { Found(v) } | (Error(_)) { Missing }
}
