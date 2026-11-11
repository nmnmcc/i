//// Finite persistent indexed deques, implemented as strict measured finger
//// trees. End edits, indexing, splitting, and concatenation take O(log(n+1))
//// node work in every history. Whole walks and bottom-up construction are O(n).
//// `equal` compares elements; ordinary `==`, inspect, and hash also observe
//// the private shape. Equal element lists need not give interchangeable dict
//// keys. Use `to_list` as a key when element equality is required.

(
	list = std/list
	option = (Option, Some, None) = std/option
)

(
	Sequence
	new
	wrap
	from_list
	to_list
	length
	is_empty
	first
	last
	push_front
	push_back
	pop_front
	pop_back
	at
	set
	append
	split
	take
	drop
	reverse
	map
	fold
	each
	equal
)

/// An immutable sequence of elements. Its constructors and cached sizes are
/// private. Old versions remain usable after every edit.
Sequence(A) {
	Sequence(Tree(A))
}

/// An empty sequence. This function may be used at any element type.
new(A) = () -> Sequence(A) { Sequence(EmptyTree) }

/// A sequence of one item.
wrap(A) = (item: A) -> Sequence(A) { Sequence(Single(Leaf(item))) }

/// Builds a sequence in O(n) work, grouping nodes from the bottom up.
from_list(A) = (items: List(A)) -> Sequence(A) {
	Sequence(build_tree(list.map(items, (x) { Leaf(x) })))
}

/// The cached number of elements, in O(1) work.
length(A) = (sequence: Sequence(A)) -> Int {
	Sequence(tree) = sequence
	tree_size(tree)
}

/// Whether there are no elements.
is_empty(A) = (sequence: Sequence(A)) -> Bool { length(sequence) == 0 }

/// The first item, or `None`.
first(A) = (sequence: Sequence(A)) -> Option(A) {
	Sequence(tree) = sequence
	tree |> {
		EmptyTree               -> None
		Single(x)               -> Some(leaf_value(x))
		Deep(_, [x, .._], _, _) -> Some(leaf_value(x))
		Deep(_, [], _, _)       -> panic("sequence: empty front digit")
	}
}

/// The last item, or `None`.
last(A) = (sequence: Sequence(A)) -> Option(A) {
	Sequence(tree) = sequence
	tree |> {
		EmptyTree -> None
		Single(x) -> Some(leaf_value(x))
		Deep(_, _, _, right) -> list.last(right) |> {
			Some(x) -> Some(leaf_value(x))
			None    -> panic("sequence: empty back digit")
		}
	}
}

/// Adds an item at the front, preserving the old sequence.
push_front(A) = (to: Sequence(A), item: A) -> Sequence(A) {
	Sequence(tree) = to
	Sequence(push_left(tree, Leaf(item)))
}

/// Adds an item at the back, preserving the old sequence.
push_back(A) = (to: Sequence(A), item: A) -> Sequence(A) {
	Sequence(tree) = to
	Sequence(push_right(tree, Leaf(item)))
}

/// Removes the first item. The item comes before the remaining sequence.
pop_front(A) = (from: Sequence(A)) -> Option((A, Sequence(A))) {
	Sequence(tree) = from
	view_left(tree) |> {
		None            -> None
		Some((x, rest)) -> Some((leaf_value(x), Sequence(rest)))
	}
}

/// Removes the last item. The item comes before the remaining sequence.
pop_back(A) = (from: Sequence(A)) -> Option((A, Sequence(A))) {
	Sequence(tree) = from
	view_right(tree) |> {
		None            -> None
		Some((x, rest)) -> Some((leaf_value(x), Sequence(rest)))
	}
}

/// The item at a zero-based index. Negative and out-of-range indices miss.
at(A) = (sequence: Sequence(A), index: Int) -> Option(A) {
	Sequence(tree) = sequence
	{ index < 0 || index >= tree_size(tree) } |> {
		True  -> None
		False -> Some(find_tree(tree, index))
	}
}

/// Replaces one item, preserving length. Invalid indices give `None`.
set(A) = (in: Sequence(A), index: Int, value: A) -> Option(Sequence(A)) {
	Sequence(tree) = in
	{ index < 0 || index >= tree_size(tree) } |> {
		True  -> None
		False -> Some(Sequence(set_tree(tree, index, value)))
	}
}

/// Concatenates two sequences by joining their fringes and sharing subtrees.
append(A) = (first: Sequence(A), second: Sequence(A)) -> Sequence(A) {
	Sequence(a) = first
	Sequence(b) = second
	Sequence(join_trees(a, [], b))
}

/// Splits before `at`, clamped to [0, length].
split(A) = (sequence: Sequence(A), at: Int) -> (Sequence(A), Sequence(A)) {
	Sequence(tree) = sequence
	at |> {
		i ? i <= 0               -> (new(), sequence)
		i ? i >= tree_size(tree) -> (sequence, new())
		i -> {
			(left, pivot, _, right) = split_tree(tree, i)
			(Sequence(left), Sequence(push_left(right, pivot)))
		}
	}
}

/// The first `up_to` items, clamping negative counts to zero.
take(A) = (from: Sequence(A), up_to: Int) -> Sequence(A) {
	(left, _) = split(from, up_to)
	left
}

/// Drops the first `up_to` items, clamping negative counts to zero.
drop(A) = (from: Sequence(A), up_to: Int) -> Sequence(A) {
	(_, right) = split(from, up_to)
	right
}

/// Reverses element order by mirroring the tree once.
reverse(A) = (sequence: Sequence(A)) -> Sequence(A) {
	Sequence(tree) = sequence
	Sequence(reverse_tree(tree))
}

/// Eagerly transforms every item, left to right, stopping on the first error.
map(A, B, E, N) = (
	over: Sequence(A)
	with: (A) -> B ^ E ! N
) -> Sequence(B) ^ E ! N {
	Sequence(tree) = over
	Sequence(map_tree(tree, with))
}

/// Combines items left to right, passing (accumulator, item) to the callback.
fold(A, Acc, E, N) = (
	over: Sequence(A)
	from: Acc
	with: (Acc, A) -> Acc ^ E ! N
) -> Acc ^ E ! N {
	Sequence(tree) = over
	fold_tree(tree, from, with)
}

/// Calls a function on each item left to right and discards its result.
each(A, B, E, N) = (over: Sequence(A), with: (A) -> B ^ E ! N) -> Nil ^ E ! N {
	fold(over, Nil, (_, x) {
		with(x)
		Nil
	})
}

/// Lists all elements in order with one tree walk.
to_list(A) = (sequence: Sequence(A)) -> List(A) {
	list.reverse(fold(sequence, [], (acc, x) { [x, ..acc] }))
}

/// Compares lengths and elements using `==`, independently of tree shape.
equal(A) = (first: Sequence(A), second: Sequence(A)) -> Bool {
	length(first) == length(second) && to_list(first) == to_list(second)
}

Node(A) {
	Leaf(A)
	Branch2(Int, Node(A), Node(A))
	Branch3(Int, Node(A), Node(A), Node(A))
}

// Digits are lists of one to four nodes of the same rank. The middle holds
// nodes one rank higher. Uniform Node/Tree types avoid polymorphic recursion.
Tree(A) {
	EmptyTree
	Single(Node(A))
	Deep(Int, List(Node(A)), Tree(A), List(Node(A)))
}

node_size(A) = (node: Node(A)) -> Int {
	node |> {
		Leaf(_)        -> 1
		Branch2(n, ..) -> n
		Branch3(n, ..) -> n
	}
}

tree_size(A) = (tree: Tree(A)) -> Int {
	tree |> {
		EmptyTree   -> 0
		Single(x)   -> node_size(x)
		Deep(n, ..) -> n
	}
}

digit_size(A) = (nodes: List(Node(A))) -> Int {
	list.fold(nodes, 0, (n, x) { n + node_size(x) })
}

branch2(A) = (a: Node(A), b: Node(A)) -> Node(A) {
	Branch2(node_size(a) + node_size(b), a, b)
}

branch3(A) = (a: Node(A), b: Node(A), c: Node(A)) -> Node(A) {
	Branch3(node_size(a) + node_size(b) + node_size(c), a, b, c)
}

deep(A) = (
	left: List(Node(A))
	middle: Tree(A)
	right: List(Node(A))
) -> Tree(A) {
	Deep(
		digit_size(left) + tree_size(middle) + digit_size(right)
		left
		middle
		right
	)
}

children(A) = (node: Node(A)) -> List(Node(A)) {
	node |> {
		Branch2(_, a, b)    -> [a, b]
		Branch3(_, a, b, c) -> [a, b, c]
		Leaf(_)             -> panic("sequence: expected a branch")
	}
}

leaf_value(A) = (node: Node(A)) -> A {
	node |> {
		Leaf(x) -> x
		_       -> panic("sequence: expected a leaf")
	}
}

// This helper is only given a bounded digit, never a whole sequence.
shallow(A) = (nodes: List(Node(A))) -> Tree(A) {
	nodes |> {
		[]           -> EmptyTree
		[x]          -> Single(x)
		[a, b]       -> deep([a], EmptyTree, [b])
		[a, b, c]    -> deep([a, b], EmptyTree, [c])
		[a, b, c, d] -> deep([a, b], EmptyTree, [c, d])
		_            -> panic("sequence: oversized digit")
	}
}

push_left(A) = (tree: Tree(A), node: Node(A)) -> Tree(A) {
	tree |> {
		EmptyTree -> Single(node)
		Single(x) -> deep([node], EmptyTree, [x])
		Deep(_, [a, b, c, d], middle, right) -> deep(
			[node, a]
			push_left(middle, branch3(b, c, d))
			right
		)
		Deep(_, left, middle, right) -> deep([node, ..left], middle, right)
	}
}

push_right(A) = (tree: Tree(A), node: Node(A)) -> Tree(A) {
	tree |> {
		EmptyTree -> Single(node)
		Single(x) -> deep([x], EmptyTree, [node])
		Deep(_, left, middle, [a, b, c, d]) -> deep(
			left
			push_right(middle, branch3(a, b, c))
			[d, node]
		)
		Deep(_, left, middle, right) -> deep(
			left
			middle
			list.append(right, [node])
		)
	}
}

left_deep(A) = (
	left: List(Node(A))
	middle: Tree(A)
	right: List(Node(A))
) -> Tree(A) {
	left |> {
		[] -> view_left(middle) |> {
			None               -> shallow(right)
			Some((node, rest)) -> deep(children(node), rest, right)
		}
		_ -> deep(left, middle, right)
	}
}

right_deep(A) = (
	left: List(Node(A))
	middle: Tree(A)
	right: List(Node(A))
) -> Tree(A) {
	right |> {
		[] -> view_right(middle) |> {
			None               -> shallow(left)
			Some((node, rest)) -> deep(left, rest, children(node))
		}
		_ -> deep(left, middle, right)
	}
}

view_left(A) = (tree: Tree(A)) -> Option((Node(A), Tree(A))) {
	tree |> {
		EmptyTree -> None
		Single(x) -> Some((x, EmptyTree))
		Deep(_, [x, ..xs], middle, right) -> Some((
			x
			left_deep(xs, middle, right)
		))
		Deep(_, [], _, _) -> panic("sequence: empty front digit")
	}
}

view_right(A) = (tree: Tree(A)) -> Option((Node(A), Tree(A))) {
	tree |> {
		EmptyTree -> None
		Single(x) -> Some((x, EmptyTree))
		Deep(_, left, middle, right) -> list.reverse(right) |> {
			[x, ..xs] -> Some((x, right_deep(left, middle, list.reverse(xs))))
			[]        -> panic("sequence: empty back digit")
		}
	}
}

// Prefer triples, replacing a remainder of one with two pairs. Join bridges
// have 2..12 nodes, and construction uses the same rule on a whole level.
group_nodes(A) = (nodes: List(Node(A))) -> List(Node(A)) {
	go = (rest, acc) {
		rest |> {
			[]              -> list.reverse(acc)
			[a, b]          -> list.reverse([branch2(a, b), ..acc])
			[a, b, c, d]    -> list.reverse([branch2(c, d), branch2(a, b), ..acc])
			[a, b, c, ..xs] -> go(xs, [branch3(a, b, c), ..acc])
			_               -> panic("sequence: ungroupable nodes")
		}
	}
	go(nodes, [])
}

join_trees(A) = (
	left: Tree(A)
	between: List(Node(A))
	right: Tree(A)
) -> Tree(A) {
	(left, right) |> {
		(EmptyTree, _) -> list.fold(list.reverse(between), right, (t, x) {
			push_left(t, x)
		})
		(_, EmptyTree) -> list.fold(between, left, (t, x) { push_right(t, x) })
		(Single(x), _) -> push_left(join_trees(EmptyTree, between, right), x)
		(_, Single(x)) -> push_right(join_trees(left, between, EmptyTree), x)
		(Deep(_, a, m, b), Deep(_, c, n, d)) -> deep(
			a
			join_trees(m, group_nodes(list.concat([b, between, c])), n)
			d
		)
	}
}

// Build each rank once. List length/split/reversal work sums geometrically.
build_tree(A) = (nodes: List(Node(A))) -> Tree(A) {
	n = list.length(nodes)
	n |> {
		count ? count <= 4 -> shallow(nodes)
		count ? count <= 8 -> {
			(left, right) = list.split(nodes, count / 2)
			deep(left, EmptyTree, right)
		}
		_ -> {
			(left, rest) = list.split(nodes, 2)
			(center, right) = list.split(rest, n - 4)
			deep(left, build_tree(group_nodes(center)), right)
		}
	}
}

// Locate a pivot and keep its residual leaf offset, so descent never restarts.
split_nodes(A) = (
	nodes: List(Node(A))
	index: Int
) -> (List(Node(A)), Node(A), Int, List(Node(A))) {
	go = (rest, offset, before) {
		rest |> {
			[] -> panic("sequence: missing pivot")
			[x, ..xs] -> { offset < node_size(x) } |> {
				True  -> (list.reverse(before), x, offset, xs)
				False -> go(xs, offset - node_size(x), [x, ..before])
			}
		}
	}
	go(nodes, index, [])
}

split_tree(A) = (
	tree: Tree(A)
	index: Int
) -> (Tree(A), Node(A), Int, Tree(A)) {
	tree |> {
		EmptyTree -> panic("sequence: split of empty tree")
		Single(x) -> (EmptyTree, x, index, EmptyTree)
		Deep(_, left, middle, right) -> {
			l = digit_size(left)
			m = tree_size(middle)
			index |> {
				i ? i < l -> {
					(before, x, offset, after) = split_nodes(left, i)
					(
						shallow(before)
						x
						offset
						left_deep(after, middle, right)
					)
				}
				i ? i < l + m -> {
					(ml, branch, residual, mr) = split_tree(middle, i - l)
					(before, x, offset, after) = split_nodes(
						children(branch)
						residual
					)
					(
						right_deep(left, ml, before)
						x
						offset
						left_deep(after, mr, right)
					)
				}
				i -> {
					(before, x, offset, after) = split_nodes(right, i - l - m)
					(
						right_deep(left, middle, before)
						x
						offset
						shallow(after)
					)
				}
			}
		}
	}
}

// Lookup follows existing list spines and branches without allocating nodes.
find_nodes(A) = (nodes: List(Node(A)), index: Int) -> A {
	nodes |> {
		[] -> panic("sequence: missing lookup node")
		[x, ..xs] -> { index < node_size(x) } |> {
			True  -> find_node(x, index)
			False -> find_nodes(xs, index - node_size(x))
		}
	}
}

find_node(A) = (node: Node(A), index: Int) -> A {
	node |> {
		Leaf(x) -> x
		Branch2(_, a, b) -> { index < node_size(a) } |> {
			True  -> find_node(a, index)
			False -> find_node(b, index - node_size(a))
		}
		Branch3(_, a, b, c) -> index |> {
			i ? i < node_size(a) -> find_node(a, i)
			i ? i < node_size(a) + node_size(b) -> find_node(
				b
				i - node_size(a)
			)
			i -> find_node(c, i - node_size(a) - node_size(b))
		}
	}
}

find_tree(A) = (tree: Tree(A), index: Int) -> A {
	tree |> {
		EmptyTree -> panic("sequence: lookup of empty tree")
		Single(x) -> find_node(x, index)
		Deep(_, left, middle, right) -> {
			l = digit_size(left)
			m = tree_size(middle)
			index |> {
				i ? i < l     -> find_nodes(left, i)
				i ? i < l + m -> find_tree(middle, i - l)
				i             -> find_nodes(right, i - l - m)
			}
		}
	}
}

set_nodes(A) = (nodes: List(Node(A)), index: Int, value: A) -> List(Node(A)) {
	(before, x, offset, after) = split_nodes(nodes, index)
	list.append(before, [set_node(x, offset, value), ..after])
}

set_node(A) = (node: Node(A), index: Int, value: A) -> Node(A) {
	node |> {
		Leaf(_) -> Leaf(value)
		Branch2(n, a, b) -> { index < node_size(a) } |> {
			True  -> Branch2(n, set_node(a, index, value), b)
			False -> Branch2(n, a, set_node(b, index - node_size(a), value))
		}
		Branch3(n, a, b, c) -> index |> {
			i ? i < node_size(a) -> Branch3(n, set_node(a, i, value), b, c)
			i ? i < node_size(a) + node_size(b) -> Branch3(
				n
				a
				set_node(b, i - node_size(a), value)
				c
			)
			i -> Branch3(
				n
				a
				b
				set_node(c, i - node_size(a) - node_size(b), value)
			)
		}
	}
}

set_tree(A) = (tree: Tree(A), index: Int, value: A) -> Tree(A) {
	tree |> {
		EmptyTree -> panic("sequence: update of empty tree")
		Single(x) -> Single(set_node(x, index, value))
		Deep(n, left, middle, right) -> {
			l = digit_size(left)
			m = tree_size(middle)
			index |> {
				i ? i < l -> Deep(n, set_nodes(left, i, value), middle, right)
				i ? i < l + m -> Deep(
					n
					left
					set_tree(middle, i - l, value)
					right
				)
				i -> Deep(n, left, middle, set_nodes(right, i - l - m, value))
			}
		}
	}
}

reverse_node(A) = (node: Node(A)) -> Node(A) {
	node |> {
		Leaf(_)          -> node
		Branch2(n, a, b) -> Branch2(n, reverse_node(b), reverse_node(a))
		Branch3(n, a, b, c) -> Branch3(
			n
			reverse_node(c)
			reverse_node(b)
			reverse_node(a)
		)
	}
}

reverse_tree(A) = (tree: Tree(A)) -> Tree(A) {
	tree |> {
		EmptyTree -> EmptyTree
		Single(x) -> Single(reverse_node(x))
		Deep(n, left, middle, right) -> Deep(
			n
			list.map(list.reverse(right), reverse_node)
			reverse_tree(middle)
			list.map(list.reverse(left), reverse_node)
		)
	}
}

map_node(A, B, E, N) = (node: Node(A), f: (A) -> B ^ E ! N) -> Node(B) ^ E ! N {
	node |> {
		Leaf(x)          -> Leaf(f(x))
		Branch2(n, a, b) -> Branch2(n, map_node(a, f), map_node(b, f))
		Branch3(n, a, b, c) -> Branch3(
			n
			map_node(a, f)
			map_node(b, f)
			map_node(c, f)
		)
	}
}

map_tree(A, B, E, N) = (tree: Tree(A), f: (A) -> B ^ E ! N) -> Tree(B) ^ E ! N {
	tree |> {
		EmptyTree -> EmptyTree
		Single(x) -> Single(map_node(x, f))
		Deep(n, left, middle, right) -> {
			a = list.map(left, (x) { map_node(x, f) })
			b = map_tree(middle, f)
			c = list.map(right, (x) { map_node(x, f) })
			Deep(n, a, b, c)
		}
	}
}

fold_node(A, Acc, E, N) = (
	node: Node(A)
	acc: Acc
	f: (Acc, A) -> Acc ^ E ! N
) -> Acc ^ E ! N {
	node |> {
		Leaf(x)          -> f(acc, x)
		Branch2(_, a, b) -> fold_node(b, fold_node(a, acc, f), f)
		Branch3(_, a, b, c) -> fold_node(
			c
			fold_node(b, fold_node(a, acc, f), f)
			f
		)
	}
}

fold_tree(A, Acc, E, N) = (
	tree: Tree(A)
	acc: Acc
	f: (Acc, A) -> Acc ^ E ! N
) -> Acc ^ E ! N {
	tree |> {
		EmptyTree -> acc
		Single(x) -> fold_node(x, acc, f)
		Deep(_, left, middle, right) -> {
			a = list.fold(left, acc, (s, x) { fold_node(x, s, f) })
			b = fold_tree(middle, a, f)
			list.fold(right, b, (s, x) { fold_node(x, s, f) })
		}
	}
}
