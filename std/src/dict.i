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

import (
	i/core
	option = (Option, Some, None) = std/option
)

export (
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
Dict(k, v) {
	Dict(Int, Node(k, v))
}

Node(k, v) {
	Empty
	One(Int, k, v)
	Many(Int, List((k, v)))
	Branch(Node(k, v), Node(k, v), Node(k, v), Node(k, v))
}

/// A dict with no entries.
new = () -> Dict(k, v) { Dict(0, Empty) }

/// The number of entries. This takes one step.
size = (dict: Dict(k, v)) -> Int {
	Dict(n, _) = dict
	n
}

/// `True` when the dict has no entries.
is_empty = (dict: Dict(k, v)) -> Bool { size(dict) == 0 }

// The two bits of a hash for a level of the tree.
slot = (hash: Int, depth: Int) -> Int {
	core.bit_and(core.shift_right(hash, depth * 2), 3)
}

/// The value for a key, or `None` when the key is not in the dict.
///
/// ```i
/// dict.new() |> dict.insert(1, "a") |> dict.get(1)  // Some("a")
/// ```
get = (from: Dict(k, v), get: k) -> Option(v) {
	Dict(_, root) = from
	find(root, core.hash(get), get, 0)
}

find = (node: Node(k, v), hash: Int, key: k, depth: Int) -> Option(v) {
	case node {
		Empty -> None
		One(h, k, v) -> case h == hash && k == key {
			True  -> Some(v)
			False -> None
		}
		Many(h, entries) -> case h == hash {
			True  -> find_entry(entries, key)
			False -> None
		}
		Branch(a, b, c, d) -> {
			child = case slot(hash, depth) {
				0 -> a
				1 -> b
				2 -> c
				_ -> d
			}
			find(child, hash, key, depth + 1)
		}
	}
}

find_entry = (entries: List((k, v)), key: k) -> Option(v) {
	case entries {
		[] -> None
		[(k, v), ..rest] -> case k == key {
			True  -> Some(v)
			False -> find_entry(rest, key)
		}
	}
}

/// `True` when the key is in the dict.
has = (dict: Dict(k, v), key: k) -> Bool {
	case get(dict, key) {
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
insert = (into: Dict(k, v), for: k, insert: v) -> Dict(k, v) {
	Dict(n, root) = into
	(node, added) = put(root, core.hash(for), for, insert, 0)
	case added {
		True  -> Dict(n + 1, node)
		False -> Dict(n, node)
	}
}

// Puts an entry in a node. Gives the new node, and whether the key is new.
put = (
	node: Node(k, v)
	hash: Int
	key: k
	value: v
	depth: Int
) -> (Node(k, v), Bool) {
	case node {
		Empty -> (One(hash, key, value), True)
		One(h, k, v) -> case h == hash {
			True -> case k == key {
				True  -> (One(hash, key, value), False)
				False -> (Many(hash, [(k, v), (key, value)]), True)
			}
			False -> (join(depth, node, h, One(hash, key, value), hash), True)
		}
		Many(h, entries) -> case h == hash {
			True -> {
				(entries, added) = put_entry(entries, key, value, [])
				(Many(h, entries), added)
			}
			False -> (join(depth, node, h, One(hash, key, value), hash), True)
		}
		Branch(a, b, c, d) -> case slot(hash, depth) {
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

put_entry = (
	entries: List((k, v))
	key: k
	value: v
	before: List((k, v))
) -> (List((k, v)), Bool) {
	case entries {
		[] -> (reverse_onto(before, [(key, value)]), True)
		[(k, v), ..rest] -> case k == key {
			True  -> (reverse_onto(before, [(key, value), ..rest]), False)
			False -> put_entry(rest, key, value, [(k, v), ..before])
		}
	}
}

// A branch with two leaves whose hashes are different.
join = (
	depth: Int
	x: Node(k, v)
	hx: Int
	y: Node(k, v)
	hy: Int
) -> Node(k, v) {
	i = slot(hx, depth)
	j = slot(hy, depth)
	case i == j {
		True  -> place(i, join(depth + 1, x, hx, y, hy), Empty)
		False -> place(j, y, place(i, x, Empty))
	}
}

// Puts a node in slot `i` of a branch. `base` is `Empty` or a branch.
place = (i: Int, node: Node(k, v), base: Node(k, v)) -> Node(k, v) {
	(a, b, c, d) = case base {
		Branch(a, b, c, d) -> (a, b, c, d)
		_                  -> (Empty, Empty, Empty, Empty)
	}
	case i {
		0 -> Branch(node, b, c, d)
		1 -> Branch(a, node, c, d)
		2 -> Branch(a, b, node, d)
		_ -> Branch(a, b, c, node)
	}
}

/// The dict without the entry for a key. If the key is not there, the dict
/// is the same.
delete = (from: Dict(k, v), delete: k) -> Dict(k, v) {
	Dict(n, root) = from
	(node, removed) = remove(root, core.hash(delete), delete, 0)
	case removed {
		True  -> Dict(n - 1, node)
		False -> from
	}
}

remove = (
	node: Node(k, v)
	hash: Int
	key: k
	depth: Int
) -> (Node(k, v), Bool) {
	case node {
		Empty -> (node, False)
		One(h, k, _) -> case h == hash && k == key {
			True  -> (Empty, True)
			False -> (node, False)
		}
		Many(h, entries) -> case h == hash {
			False -> (node, False)
			True -> case remove_entry(entries, key, []) {
				None           -> (node, False)
				Some([(k, v)]) -> (One(h, k, v), True)
				Some(rest)     -> (Many(h, rest), True)
			}
		}
		Branch(a, b, c, d) -> {
			(branch, removed) = case slot(hash, depth) {
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
			case removed {
				True  -> (shrink(branch), True)
				False -> (node, False)
			}
		}
	}
}

// A branch with only one leaf below it becomes that leaf, so that the form
// of the tree depends only on its keys.
shrink = (node: Node(k, v)) -> Node(k, v) {
	case node {
		Branch(Empty, Empty, Empty, Empty) -> Empty
		Branch(x, Empty, Empty, Empty)
			| Branch(Empty, x, Empty, Empty)
			| Branch(Empty, Empty, x, Empty)
			| Branch(Empty, Empty, Empty, x) -> case x {
			Branch(..)                       -> node
			_                                -> x
		}
		_ -> node
	}
}

remove_entry = (
	entries: List((k, v))
	key: k
	before: List((k, v))
) -> Option(List((k, v))) {
	case entries {
		[] -> None
		[(k, v), ..rest] -> case k == key {
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
upsert = (
	dict: Dict(k, v)
	in: k
	with: (Option(v)) -> v ? e ! n
) -> Dict(k, v) ? e ! n { insert(dict, in, with(get(dict, in))) }

/// Changes the value for a key with a function, when the key is there. If
/// not, the dict is the same.
update = (
	dict: Dict(k, v)
	in: k
	with: (v) -> v ? e ! n
) -> Dict(k, v) ? e ! n {
	case get(dict, in) {
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
from_list = (list: List((k, v))) -> Dict(k, v) {
	go = (rest, dict) {
		case rest {
			[]             -> dict
			[(k, v), ..xs] -> go(xs, insert(dict, k, v))
		}
	}
	go(list, new())
}

/// Combines every entry with an accumulator, as `list.fold` does. The order
/// of the entries is not fixed (see the module documentation).
fold = (
	over: Dict(k, v)
	from: acc
	with: (acc, k, v) -> acc ? e ! n
) -> acc ? e ! n {
	Dict(_, root) = over
	fold_node(root, from, with)
}

fold_node = (
	node: Node(k, v)
	acc: acc
	f: (acc, k, v) -> acc ? e ! n
) -> acc ? e ! n {
	case node {
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

fold_entries = (
	entries: List((k, v))
	acc: acc
	f: (acc, k, v) -> acc ? e ! n
) -> acc ? e ! n {
	case entries {
		[]               -> acc
		[(k, v), ..rest] -> fold_entries(rest, f(acc, k, v), f)
	}
}

/// The entries as a list of pairs, in the order of `fold`.
to_list = (dict: Dict(k, v)) -> List((k, v)) {
	reverse_onto(fold(dict, [], (acc, k, v) { [(k, v), ..acc] }), [])
}

/// The keys, in the order of `fold`.
keys = (dict: Dict(k, v)) -> List(k) {
	reverse_onto(fold(dict, [], (acc, k, _) { [k, ..acc] }), [])
}

/// The values, in the order of `fold`.
values = (dict: Dict(k, v)) -> List(v) {
	reverse_onto(fold(dict, [], (acc, _, v) { [v, ..acc] }), [])
}

/// Calls a function with every entry, in the order of `fold`.
each = (dict: Dict(k, v), with: (k, v) -> a ? e ! n) -> Nil ? e ! n {
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
map_values = (in: Dict(k, v), with: (k, v) -> w ? e ! n) -> Dict(k, w) ? e ! n {
	Dict(n, root) = in
	Dict(n, map_node(root, with))
}

map_node = (node: Node(k, v), f: (k, v) -> w ? e ! n) -> Node(k, w) ? e ! n {
	case node {
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

map_entries = (
	entries: List((k, v))
	f: (k, v) -> w ? e ! n
	acc: List((k, w))
) -> List((k, w)) ? e ! n {
	case entries {
		[]               -> reverse_onto(acc, [])
		[(k, v), ..rest] -> map_entries(rest, f, [(k, f(k, v)), ..acc])
	}
}

/// Keeps only the entries for which the function gives `True`.
filter = (
	in: Dict(k, v)
	keeping: (k, v) -> Bool ? e ! n
) -> Dict(k, v) ? e ! n {
	fold(in, in, (acc, k, v) {
		case keeping(k, v) {
			True  -> acc
			False -> delete(acc, k)
		}
	})
}

/// Keeps only the entries whose keys are in the list.
take = (from: Dict(k, v), keeping: List(k)) -> Dict(k, v) {
	go = (rest, acc) {
		case rest {
			[] -> acc
			[k, ..ks] -> case get(from, k) {
				Some(v) -> go(ks, insert(acc, k, v))
				None    -> go(ks, acc)
			}
		}
	}
	go(keeping, new())
}

/// Takes out the entries whose keys are in the list.
drop = (from: Dict(k, v), drop: List(k)) -> Dict(k, v) {
	go = (rest, acc) {
		case rest {
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
merge = (into: Dict(k, v), from: Dict(k, v)) -> Dict(k, v) {
	fold(from, into, (acc, k, v) { insert(acc, k, v) })
}

/// All the entries of both dicts. For a key that is in both, the values are
/// put together with a function of the value of `dict` and the value of
/// `other`.
combine = (
	dict: Dict(k, v)
	other: Dict(k, v)
	with: (v, v) -> v ? e ! n
) -> Dict(k, v) ? e ! n {
	fold(other, dict, (acc, k, v) {
		case get(acc, k) {
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
group = (list: List(v), by: (v) -> k ? e ! n) -> Dict(k, List(v)) ? e ! n {
	go = (rest, acc) {
		case rest {
			[] -> acc
			[x, ..xs] -> {
				key = by(x)
				go(xs, case get(acc, key) {
					Some(group) -> insert(acc, key, [x, ..group])
					None        -> insert(acc, key, [x])
				})
			}
		}
	}
	reversed = go(reverse_onto(list, []), new())
	reversed
}

reverse_onto = (list: List(a), acc: List(a)) -> List(a) {
	case list {
		[]        -> acc
		[x, ..xs] -> reverse_onto(xs, [x, ..acc])
	}
}
