//// A copy of std/dict's insert path with the per-level result tuples
//// removed, for a diagnostic comparison (never compiled into the library).
//// `put` returns only the new node; `insert` counts every insert, so the
//// size may be too large for duplicate keys. The trie shape is the same.
(
	i/core
)

(
	Dict
	new
	size
	insert
)

Dict(K, V) = (Int, Node(K, V))

Empty = ()
One(K, V) = (Int, K, V)
Many(K, V) = (Int, List((K, V)))
Branch(K, V) = (Node(K, V), Node(K, V), Node(K, V), Node(K, V))
Node(K, V) = Empty | One(K, V) | Many(K, V) | Branch(K, V)

new = <K, V>() -> Dict(K, V) { Dict(0, Empty) }

size = <K, V>(dict : Dict(K, V)) -> Int {
	Dict(n, _) = dict
	n
}

slot = (hash : Int, depth : Int) -> Int {
	core.bit_and(core.shift_right(hash, depth * 2), 3)
}

insert = <K, V>(into : Dict(K, V), for : K, insert : V) -> Dict(K, V) {
	Dict(n, root) = into
	node = put(root, core.hash(for), for, insert, 0)
	Dict(n + 1, node)
}

put = <K, V>(
	node : Node(K, V),
	hash : Int,
	key : K,
	value : V,
	depth : Int
) -> Node(K, V) {
	node |>
		(Empty) { One(hash, key, value) }
		& (One(h, k, v)) {
			{ h == hash } |>
				(True) {
					{ k == key } |>
						(True) { One(hash, key, value) }
						& (False) { Many(hash, [(k, v), (key, value)]) }
				}
				& (False) { join(depth, node, h, One(hash, key, value), hash) }
		}
		& (Many(h, entries)) {
			{ h == hash } |>
				(True) { Many(h, put_entry(entries, key, value, [])) }
				& (False) { join(depth, node, h, One(hash, key, value), hash) }
		}
		& (Branch(a, b, c, d)) {
			slot(hash, depth) |>
				(0) { Branch(put(a, hash, key, value, depth + 1), b, c, d) }
				& (1) { Branch(a, put(b, hash, key, value, depth + 1), c, d) }
				& (2) { Branch(a, b, put(c, hash, key, value, depth + 1), d) }
				& (_) { Branch(a, b, c, put(d, hash, key, value, depth + 1)) }
		}
}

put_entry = <K, V>(
	entries : List((K, V)),
	key : K,
	value : V,
	before : List((K, V))
) -> List((K, V)) {
	entries |>
		([]) { reverse_onto(before, [(key, value)]) }
		& ([(k, v), ..rest]) {
			{ k == key } |>
				(True) { reverse_onto(before, [(key, value), ..rest]) }
				& (False) { put_entry(rest, key, value, [(k, v), ..before]) }
		}
}

reverse_onto = <A>(acc : List(A), rest : List(A)) -> List(A) {
	acc |>
		([]) { rest }
		& ([x, ..xs]) { reverse_onto(xs, [x, ..rest]) }
}

join = <K, V>(
	depth : Int,
	x : Node(K, V),
	hx : Int,
	y : Node(K, V),
	hy : Int
) -> Node(K, V) {
	i = slot(hx, depth)
	j = slot(hy, depth)
	{ i == j } |>
		(True) { place(i, join(depth + 1, x, hx, y, hy), Empty) }
		& (False) { place(j, y, place(i, x, Empty)) }
}

place = <K, V>(i : Int, node : Node(K, V), base : Node(K, V)) -> Node(K, V) {
	(a, b, c, d) = base |>
		(Branch(a, b, c, d)) { (a, b, c, d) }
		& (_) { (Empty, Empty, Empty, Empty) }
	i |>
		(0) { Branch(node, b, c, d) }
		& (1) { Branch(a, node, c, d) }
		& (2) { Branch(a, b, node, d) }
		& (_) { Branch(a, b, c, node) }
}
