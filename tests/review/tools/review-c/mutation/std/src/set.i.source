//// Sets: collections of values with no two the same, which may not be
//// changed.
////
//// A `Set(a)` is a `Dict(a, Nil)` (see `std/dict`): members are compared
//// with `==` and found by their `core.hash`, so any value may be a member.
//// `insert`, `delete`, and `contains` take O(log n) steps. Two sets with the
//// same members are `==`. The order of `to_list` and `fold` is not fixed.

import (
	i/core
	dict = (Dict) = std/dict
)

export (
	Set
	new
	size
	is_empty
	insert
	delete
	contains
	to_list
	from_list
	fold
	each
	map
	filter
	take
	drop
	union
	intersection
	difference
	symmetric_difference
	is_subset
	is_disjoint
)

/// A set of values of type `a`.
Set(a) {
	Set(Dict(a, Nil))
}

/// A set with no members.
new = () -> Set(a) { Set(dict.new()) }

/// The number of members. This takes one step.
size = (set: Set(a)) -> Int {
	Set(d) = set
	dict.size(d)
}

/// `True` when the set has no members.
is_empty = (set: Set(a)) -> Bool { size(set) == 0 }

/// The set with a new member. If it is already there, the set is the same.
insert = (into: Set(a), this: a) -> Set(a) {
	Set(d) = into
	Set(dict.insert(d, this, Nil))
}

/// The set without a member.
delete = (from: Set(a), this: a) -> Set(a) {
	Set(d) = from
	Set(dict.delete(d, this))
}

/// `True` when `this` is a member.
contains = (in: Set(a), this: a) -> Bool {
	Set(d) = in
	dict.has(d, this)
}

/// The members, in the order of `fold`.
to_list = (set: Set(a)) -> List(a) {
	Set(d) = set
	dict.keys(d)
}

/// A set of the elements of a list.
///
/// ```i
/// set.from_list([1, 2, 1]) |> set.size  // 2
/// ```
from_list = (members: List(a)) -> Set(a) {
	go = (rest, d) {
		case rest {
			[]        -> Set(d)
			[x, ..xs] -> go(xs, dict.insert(d, x, Nil))
		}
	}
	go(members, dict.new())
}

/// Combines every member with an accumulator. The order is not fixed.
fold = (over: Set(a), from: acc, with: (acc, a) -> acc ? e ! n) -> acc ? e ! n {
	Set(d) = over
	dict.fold(d, from, (acc, k, _) { with(acc, k) })
}

/// Calls a function with every member.
each = (set: Set(a), with: (a) -> b ? e ! n) -> Nil ? e ! n {
	fold(set, Nil, (_, x) {
		with(x)
		Nil
	})
}

/// The set of the values of a function on every member.
map = (set: Set(a), with: (a) -> b ? e ! n) -> Set(b) ? e ! n {
	fold(set, new(), (acc, x) { insert(acc, with(x)) })
}

/// Keeps the members for which the function gives `True`.
filter = (in: Set(a), keeping: (a) -> Bool ? e ! n) -> Set(a) ? e ! n {
	Set(d) = in
	Set(dict.filter(d, (k, _) { keeping(k) }))
}

/// Keeps only the members that are in the list.
take = (from: Set(a), keeping: List(a)) -> Set(a) {
	Set(d) = from
	Set(dict.take(d, keeping))
}

/// Takes out the members that are in the list.
drop = (from: Set(a), drop: List(a)) -> Set(a) {
	Set(d) = from
	Set(dict.drop(d, drop))
}

/// The members of one or both sets.
union = (of: Set(a), and: Set(a)) -> Set(a) {
	(small, big) = by_size(of, and)
	fold(small, big, insert)
}

/// The members of both sets.
intersection = (of: Set(a), and: Set(a)) -> Set(a) {
	(small, big) = by_size(of, and)
	filter(small, (x) { contains(big, x) })
}

/// The members of `from` that are not in `minus`.
difference = (from: Set(a), minus: Set(a)) -> Set(a) {
	filter(from, (x) { core.not(contains(minus, x)) })
}

/// The members of exactly one of the sets.
symmetric_difference = (of: Set(a), and: Set(a)) -> Set(a) {
	union(difference(of, and), difference(and, of))
}

/// `True` when every member of `first` is in `of`.
is_subset = (first: Set(a), of: Set(a)) -> Bool {
	size(first) <= size(of) && fold(first, True, (ok, x) {
		ok && contains(of, x)
	})
}

/// `True` when no member is in both sets.
is_disjoint = (first: Set(a), from: Set(a)) -> Bool {
	(small, big) = by_size(first, from)
	fold(small, True, (ok, x) { ok && core.not(contains(big, x)) })
}

by_size = (a: Set(x), b: Set(x)) -> (Set(x), Set(x)) {
	case size(a) <= size(b) {
		True  -> (a, b)
		False -> (b, a)
	}
}
