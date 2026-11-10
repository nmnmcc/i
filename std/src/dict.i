//// Dictionaries: maps from keys to values that may not be changed.
////
//// A `Dict(k, v)` is a hash trie. The key of every entry is hashed with
//// `core.hash` (§13), and the hash picks a path through a tree with four
//// branches at each level, two bits of the hash for each level. Keys are
//// compared with `==` (§11.4), so any value may be a key and no compare
//// function is needed. `get`, `insert`, and `delete` take O(log n) steps.
////
//// The tree of a set of keys has only one form, whatever order the keys
//// were put in. So two dicts with the same entries are `==`. (The one
//// exception: keys with the same 32-bit hash are kept in the order they were
//// put in, so two such dicts may differ in that order.)
////
//// The order of `keys`, `values`, `to_list`, and `fold` comes from the
//// hashes. It is the same every time for the same compiled program, but
//// it is not sorted, and it is not fixed by this library. Sort the result
//// of `to_list` when an order is needed.

(
	i/core
	option = (Option, Some, None) = std/option
)

(
	Dict
	new
	size
	is_empty
	get
	has
	insert
	delete
	upsert
	update
	from_list
	to_list
	keys
	values
	fold
	each
	map_values
	filter
	take
	drop
	merge
	combine
	group
)

/// A map from keys of type `k` to values of type `v`.
Dict(K, V) {
	Dict(Int, Node(K, V))
}

Node(K, V) {
	Empty
	One(Int, K, V)
	Many(Int, List((K, V)))
	Branch(Node(K, V), Node(K, V), Node(K, V), Node(K, V))
}

/// A dict with no entries.
new(K, V) = () -> Dict(K, V) { Dict(0, Empty) }

/// The number of entries. This takes one step.
size(K, V) = (dict: Dict(K, V)) -> Int {
	Dict(n, _) = dict
	n
}

/// `True` when the dict has no entries.
is_empty(K, V) = (dict: Dict(K, V)) -> Bool { size(dict) == 0 }

// The two bits of a hash for a level of the tree.
slot = (hash: Int, depth: Int) -> Int {
	core.bit_and(core.shift_right(hash, depth * 2), 3)
}

/// The value for a key, or `None` when the key is not in the dict.
///
/// ```i
/// dict.new() |> dict.insert(1, "a") |> dict.get(1)  // Some("a")
/// ```
get(K, V) = (from: Dict(K, V), get: K) -> Option(V) {
	Dict(_, root) = from
	find(root, core.hash(get), get, 0)
}

find(K, V) = (node: Node(K, V), hash: Int, key: K, depth: Int) -> Option(V) {
	node |> {
		Empty -> None
		One(h, k, v) -> { h == hash && k == key } |> {
			True  -> Some(v)
			False -> None
		}
		Many(h, entries) -> { h == hash } |> {
			True  -> find_entry(entries, key)
			False -> None
		}
		Branch(a, b, c, d) -> {
			child = slot(hash, depth) |> {
				0 -> a
				1 -> b
				2 -> c
				_ -> d
			}
			find(child, hash, key, depth + 1)
		}
	}
}

find_entry(K, V) = (entries: List((K, V)), key: K) -> Option(V) {
	entries |> {
		[] -> None
		[(k, v), ..rest] -> { k == key } |> {
			True  -> Some(v)
			False -> find_entry(rest, key)
		}
	}
}

/// `True` when the key is in the dict.
has(K, V) = (dict: Dict(K, V), key: K) -> Bool {
	get(dict, key) |> {
		Some(_) -> True
		None    -> False
	}
}

/// The dict with the entry `key` → `value`. An entry with the same key is
/// replaced.
///
/// ```i
/// dict.new() |> dict.insert("a", 1) |> dict.to_list  // [("a", 1)]
/// ```
insert(K, V) = (into: Dict(K, V), for: K, insert: V) -> Dict(K, V) {
	Dict(n, root) = into
	(node, added) = put(root, core.hash(for), for, insert, 0)
	added |> {
		True  -> Dict(n + 1, node)
		False -> Dict(n, node)
	}
}

// Puts an entry in a node. Gives the new node, and whether the key is new.
put(K, V) = (
	node: Node(K, V)
	hash: Int
	key: K
	value: V
	depth: Int
) -> (Node(K, V), Bool) {
	node |> {
		Empty -> (One(hash, key, value), True)
		One(h, k, v) -> { h == hash } |> {
			True -> { k == key } |> {
				True  -> (One(hash, key, value), False)
				False -> (Many(hash, [(k, v), (key, value)]), True)
			}
			False -> (join(depth, node, h, One(hash, key, value), hash), True)
		}
		Many(h, entries) -> { h == hash } |> {
			True -> {
				(entries, added) = put_entry(entries, key, value, [])
				(Many(h, entries), added)
			}
			False -> (join(depth, node, h, One(hash, key, value), hash), True)
		}
		Branch(a, b, c, d) -> slot(hash, depth) |> {
			0 -> {
				(x, added) = put(a, hash, key, value, depth + 1)
				(Branch(x, b, c, d), added)
			}
			1 -> {
				(x, added) = put(b, hash, key, value, depth + 1)
				(Branch(a, x, c, d), added)
			}
			2 -> {
				(x, added) = put(c, hash, key, value, depth + 1)
				(Branch(a, b, x, d), added)
			}
			_ -> {
				(x, added) = put(d, hash, key, value, depth + 1)
				(Branch(a, b, c, x), added)
			}
		}
	}
}

put_entry(K, V) = (
	entries: List((K, V))
	key: K
	value: V
	before: List((K, V))
) -> (List((K, V)), Bool) {
	entries |> {
		[] -> (reverse_onto(before, [(key, value)]), True)
		[(k, v), ..rest] -> { k == key } |> {
			True  -> (reverse_onto(before, [(key, value), ..rest]), False)
			False -> put_entry(rest, key, value, [(k, v), ..before])
		}
	}
}

// A branch with two leaves whose hashes are different.
join(K, V) = (
	depth: Int
	x: Node(K, V)
	hx: Int
	y: Node(K, V)
	hy: Int
) -> Node(K, V) {
	i = slot(hx, depth)
	j = slot(hy, depth)
	{ i == j } |> {
		True  -> place(i, join(depth + 1, x, hx, y, hy), Empty)
		False -> place(j, y, place(i, x, Empty))
	}
}

// Puts a node in slot `i` of a branch. `base` is `Empty` or a branch.
place(K, V) = (i: Int, node: Node(K, V), base: Node(K, V)) -> Node(K, V) {
	(a, b, c, d) = base |> {
		Branch(a, b, c, d) -> (a, b, c, d)
		_                  -> (Empty, Empty, Empty, Empty)
	}
	i |> {
		0 -> Branch(node, b, c, d)
		1 -> Branch(a, node, c, d)
		2 -> Branch(a, b, node, d)
		_ -> Branch(a, b, c, node)
	}
}

/// The dict without the entry for a key. If the key is not there, the dict
/// is the same.
delete(K, V) = (from: Dict(K, V), delete: K) -> Dict(K, V) {
	Dict(n, root) = from
	(node, removed) = remove(root, core.hash(delete), delete, 0)
	removed |> {
		True  -> Dict(n - 1, node)
		False -> from
	}
}

remove(K, V) = (
	node: Node(K, V)
	hash: Int
	key: K
	depth: Int
) -> (Node(K, V), Bool) {
	node |> {
		Empty -> (node, False)
		One(h, k, _) -> { h == hash && k == key } |> {
			True  -> (Empty, True)
			False -> (node, False)
		}
		Many(h, entries) -> { h == hash } |> {
			False -> (node, False)
			True -> remove_entry(entries, key, []) |> {
				None           -> (node, False)
				Some([(k, v)]) -> (One(h, k, v), True)
				Some(rest)     -> (Many(h, rest), True)
			}
		}
		Branch(a, b, c, d) -> {
			(branch, removed) = slot(hash, depth) |> {
				0 -> {
					(x, r) = remove(a, hash, key, depth + 1)
					(Branch(x, b, c, d), r)
				}
				1 -> {
					(x, r) = remove(b, hash, key, depth + 1)
					(Branch(a, x, c, d), r)
				}
				2 -> {
					(x, r) = remove(c, hash, key, depth + 1)
					(Branch(a, b, x, d), r)
				}
				_ -> {
					(x, r) = remove(d, hash, key, depth + 1)
					(Branch(a, b, c, x), r)
				}
			}
			removed |> {
				True  -> (shrink(branch), True)
				False -> (node, False)
			}
		}
	}
}

// A branch with only one leaf below it becomes that leaf, so that the form
// of the tree depends only on its keys.
shrink(K, V) = (node: Node(K, V)) -> Node(K, V) {
	node |> {
		Branch(Empty, Empty, Empty, Empty) -> Empty
		Branch(x, Empty, Empty, Empty)
			| Branch(Empty, x, Empty, Empty)
			| Branch(Empty, Empty, x, Empty)
			| Branch(Empty, Empty, Empty, x) -> x |> {
			Branch(..)                       -> node
			_                                -> x
		}
		_ -> node
	}
}

remove_entry(K, V) = (
	entries: List((K, V))
	key: K
	before: List((K, V))
) -> Option(List((K, V))) {
	entries |> {
		[] -> None
		[(k, v), ..rest] -> { k == key } |> {
			True  -> Some(reverse_onto(before, rest))
			False -> remove_entry(rest, key, [(k, v), ..before])
		}
	}
}

/// Changes the value for a key with a function, which gets `Some(value)`
/// when the key is there and `None` when not.
///
/// ```i
/// increment = (x) { case x { Some(n) -> n + 1  None -> 0 } }
/// dict.upsert(counts, in: "a", with: increment)
/// ```
upsert(K, V, E, N) = (
	dict: Dict(K, V)
	in: K
	with: (Option(V)) -> V ? E ! N
) -> Dict(K, V) ? E ! N { insert(dict, in, with(get(dict, in))) }

/// Changes the value for a key with a function, when the key is there. If
/// not, the dict is the same.
update(K, V, E, N) = (
	dict: Dict(K, V)
	in: K
	with: (V) -> V ? E ! N
) -> Dict(K, V) ? E ! N {
	get(dict, in) |> {
		Some(value) -> insert(dict, in, with(value))
		None        -> dict
	}
}

/// A dict with the entries of a list of pairs. When a key is in the list
/// more than one time, the last value is kept.
///
/// ```i
/// dict.from_list([("a", 1), ("b", 2)]) |> dict.get("b")  // Some(2)
/// ```
from_list(K, V) = (list: List((K, V))) -> Dict(K, V) {
	go = (rest, dict) {
		rest |> {
			[]             -> dict
			[(k, v), ..xs] -> go(xs, insert(dict, k, v))
		}
	}
	go(list, new())
}

/// Combines every entry with an accumulator, as `list.fold` does. The order
/// of the entries is not fixed (see the module documentation).
fold(K, V, Acc, E, N) = (
	over: Dict(K, V)
	from: Acc
	with: (Acc, K, V) -> Acc ? E ! N
) -> Acc ? E ! N {
	Dict(_, root) = over
	fold_node(root, from, with)
}

fold_node(K, V, Acc, E, N) = (
	node: Node(K, V)
	acc: Acc
	f: (Acc, K, V) -> Acc ? E ! N
) -> Acc ? E ! N {
	node |> {
		Empty            -> acc
		One(_, k, v)     -> f(acc, k, v)
		Many(_, entries) -> fold_entries(entries, acc, f)
		Branch(a, b, c, d) -> fold_node(
			d
			fold_node(c, fold_node(b, fold_node(a, acc, f), f), f)
			f
		)
	}
}

fold_entries(K, V, Acc, E, N) = (
	entries: List((K, V))
	acc: Acc
	f: (Acc, K, V) -> Acc ? E ! N
) -> Acc ? E ! N {
	entries |> {
		[]               -> acc
		[(k, v), ..rest] -> fold_entries(rest, f(acc, k, v), f)
	}
}

/// The entries as a list of pairs, in the order of `fold`.
to_list(K, V) = (dict: Dict(K, V)) -> List((K, V)) {
	reverse_onto(fold(dict, [], (acc, k, v) { [(k, v), ..acc] }), [])
}

/// The keys, in the order of `fold`.
keys(K, V) = (dict: Dict(K, V)) -> List(K) {
	reverse_onto(fold(dict, [], (acc, k, _) { [k, ..acc] }), [])
}

/// The values, in the order of `fold`.
values(K, V) = (dict: Dict(K, V)) -> List(V) {
	reverse_onto(fold(dict, [], (acc, _, v) { [v, ..acc] }), [])
}

/// Calls a function with every entry, in the order of `fold`.
each(K, V, A, E, N) = (
	dict: Dict(K, V)
	with: (K, V) -> A ? E ! N
) -> Nil ? E ! N {
	fold(dict, Nil, (_, k, v) {
		with(k, v)
		Nil
	})
}

/// Changes every value with a function of its key and value. The keys stay.
///
/// ```i
/// dict.from_list([(1, 2)]) |> dict.map_values((k, v) { k + v })  // {1 -> 3}
/// ```
map_values(K, V, W, E, N) = (
	in: Dict(K, V)
	with: (K, V) -> W ? E ! N
) -> Dict(K, W) ? E ! N {
	Dict(n, root) = in
	Dict(n, map_node(root, with))
}

map_node(K, V, W, E, N) = (
	node: Node(K, V)
	f: (K, V) -> W ? E ! N
) -> Node(K, W) ? E ! N {
	node |> {
		Empty            -> Empty
		One(h, k, v)     -> One(h, k, f(k, v))
		Many(h, entries) -> Many(h, map_entries(entries, f, []))
		Branch(a, b, c, d) -> {
			a2 = map_node(a, f)
			b2 = map_node(b, f)
			c2 = map_node(c, f)
			d2 = map_node(d, f)
			Branch(a2, b2, c2, d2)
		}
	}
}

map_entries(K, V, W, E, N) = (
	entries: List((K, V))
	f: (K, V) -> W ? E ! N
	acc: List((K, W))
) -> List((K, W)) ? E ! N {
	entries |> {
		[]               -> reverse_onto(acc, [])
		[(k, v), ..rest] -> map_entries(rest, f, [(k, f(k, v)), ..acc])
	}
}

/// Keeps only the entries for which the function gives `True`.
filter(K, V, E, N) = (
	in: Dict(K, V)
	keeping: (K, V) -> Bool ? E ! N
) -> Dict(K, V) ? E ! N {
	fold(in, in, (acc, k, v) {
		keeping(k, v) |> {
			True  -> acc
			False -> delete(acc, k)
		}
	})
}

/// Keeps only the entries whose keys are in the list.
take(K, V) = (from: Dict(K, V), keeping: List(K)) -> Dict(K, V) {
	go = (rest, acc) {
		rest |> {
			[] -> acc
			[k, ..ks] -> get(from, k) |> {
				Some(v) -> go(ks, insert(acc, k, v))
				None    -> go(ks, acc)
			}
		}
	}
	go(keeping, new())
}

/// Takes out the entries whose keys are in the list.
drop(K, V) = (from: Dict(K, V), drop: List(K)) -> Dict(K, V) {
	go = (rest, acc) {
		rest |> {
			[]        -> acc
			[k, ..ks] -> go(ks, delete(acc, k))
		}
	}
	go(drop, from)
}

/// All the entries of both dicts. For a key that is in both, the value of
/// `from` is kept.
///
/// ```i
/// dict.merge(dict.from_list([(1, "a")]), dict.from_list([(1, "b"), (2, "c")]))
/// // {1 -> "b", 2 -> "c"}
/// ```
merge(K, V) = (into: Dict(K, V), from: Dict(K, V)) -> Dict(K, V) {
	fold(from, into, (acc, k, v) { insert(acc, k, v) })
}

/// All the entries of both dicts. For a key that is in both, the values are
/// put together with a function of the value of `dict` and the value of
/// `other`.
combine(K, V, E, N) = (
	dict: Dict(K, V)
	other: Dict(K, V)
	with: (V, V) -> V ? E ! N
) -> Dict(K, V) ? E ! N {
	fold(other, dict, (acc, k, v) {
		get(acc, k) |> {
			Some(old) -> insert(acc, k, with(old, v))
			None      -> insert(acc, k, v)
		}
	})
}

/// Puts the values of a list into groups by a key function. Each group
/// keeps the values in the order of the list.
///
/// ```i
/// dict.group([1, 2, 3, 4], by: (x) { x % 2 })  // {1 -> [1, 3], 0 -> [2, 4]}
/// ```
group(V, K, E, N) = (
	list: List(V)
	by: (V) -> K ? E ! N
) -> Dict(K, List(V)) ? E ! N {
	go = (rest, acc) {
		rest |> {
			[] -> acc
			[x, ..xs] -> {
				key = by(x)
				go(xs, get(acc, key) |> {
					Some(group) -> insert(acc, key, [x, ..group])
					None        -> insert(acc, key, [x])
				})
			}
		}
	}
	reversed = go(reverse_onto(list, []), new())
	reversed
}

reverse_onto(A) = (list: List(A), acc: List(A)) -> List(A) {
	list |> {
		[]        -> acc
		[x, ..xs] -> reverse_onto(xs, [x, ..acc])
	}
}
