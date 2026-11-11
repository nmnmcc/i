//// Structural record operations (§4.9).
(get, put, remove, has, keys, size, texts)

/// Read a field identified by one known key.
@external("ilang:record", "get")
get(K, V): (r: (K: V), key: K) -> V

/// Set a field, replacing any previous value.
@external("ilang:record", "put")
put(R, K, V): (r: R & Record, key: K, value: V) -> (..R, K: V)

/// Remove a field if it is present.
@external("ilang:record", "remove")
remove(R, K): (r: R & Record, key: K) -> (..R, -K)

/// Test whether a field exists, including fields hidden by the static type.
@external("ilang:record", "has")
has: (r: Record, key: String) -> Bool

/// Field labels in byte order.
@external("ilang:record", "keys")
keys: (r: Record) -> List(String)

/// Number of fields.
@external("ilang:record", "size")
size: (r: Record) -> Int

/// Field labels and value text in byte order.
@external("ilang:record", "texts")
texts: (r: Record) -> List((String, String))
