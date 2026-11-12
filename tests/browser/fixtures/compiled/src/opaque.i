//// Opaque boundary regressions for spec 4.1 and 11.4.
(
	i/core
	app/other
)

(
	A
	B
	SyncA
	SyncB
	AsyncA
	AsyncB
	direct_aa
	direct_ab
	generic_aa
	generic_ab
	alias_aa
	module_ab
	collection_aa
	collection_ab
	hash_aa
	inspect_a
	echo
	roundtrip_aa
	roundtrip_ab
	import_aa
	import_ab
	sync_aa
	sync_ab
	async_aa
	async_ab
)

A = Opaque

B = Opaque

Alias = A

@host("app:opaque-sync/v1")
SyncA = (get_a = () -> A)

@host("app:opaque-sync/v1")
SyncB = (get_b = () -> B)

@host("app:opaque-async/v1", "async")
AsyncA = (get_a = () -> A)

@host("app:opaque-async/v1", "async")
AsyncB = (get_b = () -> B)

@external("app:opaque/v1", "get")
get_a: () -> A

@external("app:opaque/v1", "get")
get_b: () -> B

@external("app:opaque/v1", "echo")
echo_a: (A) -> A

@external("app:opaque/v1", "echo")
echo_b: (A) -> B

equal = <X, Y>(a: X, b: Y) { a == b }

direct_aa = (a: A, b: A) { a == b }

// Section 6.9 rejects equality of known disjoint types. Widen the local
// known types so this direct operator reaches runtime equality.
direct_ab = (a: A, b: B) {
	left: A | B = a
	right: A | B = b
	left == right
}

generic_aa = (a: A, b: A) { equal(a, b) }

generic_ab = (a: A, b: B) { equal(a, b) }

alias_aa = (a: A, b: Alias) { equal(a, b) }

module_ab = (a: A, b: other.A) { equal(a, b) }

collection_aa = (a: A, b: A) { equal([a], [b]) }

collection_ab = (a: A, b: B) { equal([a], [b]) }

hash_aa = (a: A, b: A) { core.hash(a) == core.hash(b) }

inspect_a = (a: A) { core.inspect(a) }

echo = (a: A) { a }

roundtrip_aa = (a: A, b: A) { equal(a, echo_a(b)) }

roundtrip_ab = (a: A) { equal(a, echo_b(a)) }

import_aa = () { equal(get_a(), get_a()) }

import_ab = () { equal(get_a(), get_b()) }

sync_aa = () <- (a: SyncA) { equal(a.get_a(), a.get_a()) }

sync_ab = () <- (a: SyncA, b: SyncB) { equal(a.get_a(), b.get_b()) }

async_aa = () <- (a: AsyncA) { equal(a.get_a(), a.get_a()) }

async_ab = () <- (a: AsyncA, b: AsyncB) { equal(a.get_a(), b.get_b()) }
