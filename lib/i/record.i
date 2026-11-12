//// Structural record operations (§4.9).
(
	get
	put
	remove
	has
	keys
	size
	texts
)

/// Read a field identified by one known key.
@external("ilang:record", "get")
get: <K, V>((k = V), K) -> V

/// Set a field, replacing any previous value.
@external("ilang:record", "put")
put: <R, K, V>(R & Record, K, V) -> (k = V)

/// Remove a field if it is present.
@external("ilang:record", "remove")
remove: <R, K>(R & Record, K) -> ()

/// Test whether a field exists, including fields hidden by the static type.
@external("ilang:record", "has")
has: (Record, String) -> Bool

/// Field labels in byte order.
@external("ilang:record", "keys")
keys: (Record) -> List(String)

/// Number of fields.
@external("ilang:record", "size")
size: (Record) -> Int

/// Field labels and value text in byte order.
@external("ilang:record", "texts")
texts: (Record) -> List((String, String))
