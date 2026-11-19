//// Sets: collections of values with no two the same, which may not be
//// changed.
////
//// A `Set(A)` is a `Dict(A, ())` (see `std/dict`): members are compared
//// with `==` and found by their `core.hash`, so any value may be a member.
//// `insert`, `delete`, and `contains` take O(log n) steps. The order of
//// `to_list` and `fold` is not fixed.
////
//// `==` compares the representation, not the members: members with the same
//// 32-bit hash are kept in the order they were put in, so two sets with the
//// same members may differ under `==`. Use `same_elements` to compare
//// members.
(
	i/core
	dict = (Dict) = std/dict
)

(
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
	same_elements
)

/// A set of values of type `a`.
Set(A) = (Dict(A, ()))

/// A set with no members.
new = <A>() -> Set(A) { Set(dict.new()) }

/// The number of members. This takes one step.
size = <A>(set : Set(A)) -> Int {
	Set(d) = set
	dict.size(d)
}

/// `True` when the set has no members.
is_empty = <A>(set : Set(A)) -> Bool { size(set) == 0 }

/// The set with a new member. If it is already there, the set is the same.
insert = <A>(into : Set(A), this : A) -> Set(A) {
	Set(d) = into
	Set(dict.insert(d, this, ()))
}

/// The set without a member.
delete = <A>(from : Set(A), this : A) -> Set(A) {
	Set(d) = from
	Set(dict.delete(d, this))
}

/// `True` when `this` is a member.
contains = <A>(in : Set(A), this : A) -> Bool {
	Set(d) = in
	dict.has(d, this)
}

/// The members, in the order of `fold`.
to_list = <A>(set : Set(A)) -> List(A) {
	Set(d) = set
	dict.keys(d)
}

/// A set of the elements of a list.
///
/// ```i
/// set.from_list([1, 2, 1]) |> set.size  // 2
/// ```
from_list = <A>(members : List(A)) -> Set(A) {
	go = (rest, d) {
		rest |> ([]) { Set(d) } & ([x, ..xs]) { go(xs, dict.insert(d, x, ())) }
	}
	go(members, dict.new())
}

/// Combines every member with an accumulator. The order is not fixed.
fold = <A, Acc, N>(
	over : Set(A),
	from : Acc,
	with : (Acc, A) <- N -> Acc
) -> Acc {
	Set(d) = over
	dict.fold(d, from, (acc, k, _) { with(acc, k) })
}

/// Calls a function with every member.
each = <A, B, N>(set : Set(A), with : (A) <- N -> B) -> () {
	fold(set, (), (_, x) {
		with(x)
		()
	})
}

/// The set of the values of a function on every member.
map = <A, B, N>(set : Set(A), with : (A) <- N -> B) -> Set(B) {
	fold(set, new(), (acc, x) { insert(acc, with(x)) })
}

/// Keeps the members for which the function gives `True`.
filter = <A, N>(in : Set(A), keeping : (A) <- N -> Bool) -> Set(A) {
	Set(d) = in
	Set(dict.filter(d, (k, _) { keeping(k) }))
}

/// Keeps only the members that are in the list.
take = <A>(from : Set(A), keeping : List(A)) -> Set(A) {
	Set(d) = from
	Set(dict.take(d, keeping))
}

/// Takes out the members that are in the list.
drop = <A>(from : Set(A), drop : List(A)) -> Set(A) {
	Set(d) = from
	Set(dict.drop(d, drop))
}

/// The members of one or both sets.
union = <A>(of : Set(A), and : Set(A)) -> Set(A) {
	(small, big) = by_size(of, and)
	fold(small, big, insert)
}

/// The members of both sets.
intersection = <A>(of : Set(A), and : Set(A)) -> Set(A) {
	(small, big) = by_size(of, and)
	filter(small, (x) { contains(big, x) })
}

/// The members of `from` that are not in `minus`.
difference = <A>(from : Set(A), minus : Set(A)) -> Set(A) {
	filter(from, (x) { core.not(contains(minus, x)) })
}

/// The members of exactly one of the sets.
symmetric_difference = <A>(of : Set(A), and : Set(A)) -> Set(A) {
	union(difference(of, and), difference(and, of))
}

/// `True` when every member of `first` is in `of`.
is_subset = <A>(first : Set(A), of : Set(A)) -> Bool {
	size(first) <= size(of) &&
		fold(first, True, (ok, x) { ok && contains(of, x) })
}

/// `True` when no member is in both sets.
is_disjoint = <A>(first : Set(A), from : Set(A)) -> Bool {
	(small, big) = by_size(first, from)
	fold(small, True, (ok, x) { ok && core.not(contains(big, x)) })
}

/// `True` when both sets have the same members, whatever order they were put
/// in. This takes O(n log n) steps (see `dict.same_entries`).
///
/// ```i
/// set.same_elements(set.from_list([1, 2]), set.from_list([2, 1]))  // True
/// ```
same_elements = <A>(first : Set(A), second : Set(A)) -> Bool {
	Set(a) = first
	Set(b) = second
	dict.same_entries(a, b)
}

by_size = <X>(a : Set(X), b : Set(X)) -> (Set(X), Set(X)) {
	{ size(a) <= size(b) } |> (True) { (a, b) } & (False) { (b, a) }
}
