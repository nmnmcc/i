//// Functions for `List(A)`, the built-in type of lists that may not be
//// changed.
////
//// Every function that walks a whole list is tail-recursive (§11.6), so it
//// works on lists of any length without using more stack. Functions that
//// build a new list build it in reverse and then turn it round, which takes
//// one more pass.
////
//// Functions that may find nothing give an `Option` (`first`, `find`,
//// `at`, …). Functions that take a function as an argument have the errors
//// and the environment of that function (`? E ! N`), so the argument may
//// itself raise errors or need services.
(
	dict = (Dict) = std/dict
	option = (Option, Some, None) = std/option
	order = (Order, Less, Equal, Greater) = std/order
)

(
	ContinueOrStop(..)
	length
	count
	reverse
	is_empty
	contains
	first
	rest
	last
	at
	index_of
	group
	filter
	filter_map
	map
	map2
	map_fold
	index_map
	try_map
	drop
	take
	new
	wrap
	append
	prepend
	concat
	flat_map
	fold
	fold_right
	index_fold
	try_fold
	fold_until
	find
	find_map
	all
	any
	zip
	strict_zip
	unzip
	intersperse
	unique
	sort
	range
	repeat
	split
	split_while
	key_find
	key_filter
	key_pop
	key_set
	each
	try_each
	partition
	permutations
	window
	window_by_2
	drop_while
	take_while
	chunk
	sized_chunk
	reduce
	scan
	combinations
	combination_pairs
	interleave
	transpose
	max
	min
)

Continue(A) = (A)
Stop(A) = (A)
/// What `fold_until` does next: go on with the new accumulator, or stop
/// with it.
ContinueOrStop(A) = Continue(A) | Stop(A)

/// The number of elements. It walks the whole list.
///
/// ```i
/// list.length([1, 2, 3])  // 3
/// ```
length = <A>(of : List(A)) -> Int {
	go = (rest, n) { rest |> ([]) { n } | ([_, ..xs]) { go(xs, n + 1) } }
	go(of, 0)
}

/// The number of elements for which the function gives `True`.
count = <A, N>(list : List(A), where : (A) <- N -> Bool) -> Int {
	go = (rest, n) {
		rest |>
			([]) { n }
			| ([x, ..xs]) {
				where(x) |> (True) { go(xs, n + 1) } | (False) { go(xs, n) }
			}
	}
	go(list, 0)
}

/// The elements in the other order.
///
/// ```i
/// list.reverse([1, 2, 3])  // [3, 2, 1]
/// ```
reverse = <A>(list : List(A)) -> List(A) { reverse_onto(list, []) }

// Puts the elements of `list`, last first, in front of `acc`.
reverse_onto = <A>(list : List(A), acc : List(A)) -> List(A) {
	list |> ([]) { acc } | ([x, ..xs]) { reverse_onto(xs, [x, ..acc]) }
}

/// `True` for the empty list. It takes one step.
is_empty = <A>(list : List(A)) -> Bool { list == [] }

/// `True` when an element is `==` to `any`.
///
/// ```i
/// list.contains([1, 2], 2)  // True
/// ```
contains = <A>(list : List(A), any : A) -> Bool {
	list |> ([]) { False } | ([x, ..xs]) { x == any || contains(xs, any) }
}

/// The first element, or `None` for the empty list.
first = <A>(list : List(A)) -> Option(A) {
	list |> ([]) { None } | ([x, ..]) { Some(x) }
}

/// The list without its first element, or `None` for the empty list.
rest = <A>(list : List(A)) -> Option(List(A)) {
	list |> ([]) { None } | ([_, ..xs]) { Some(xs) }
}

/// The last element, or `None` for the empty list.
last = <A>(list : List(A)) -> Option(A) {
	list |> ([]) { None } | ([x]) { Some(x) } | ([_, ..xs]) { last(xs) }
}

/// The element at an index, counting from 0, or `None` when the index is
/// not in the list.
///
/// ```i
/// list.at([10, 20, 30], 1)  // Some(20)
/// ```
at = <A>(list : List(A), index : Int) -> Option(A) {
	{ index < 0 } |>
		(True) { None }
		| (False) {
			list |>
				([]) { None }
				| ([x, ..xs]) {
					index |> (0) { Some(x) } | (_) { at(xs, index - 1) }
				}
		}
}

/// The index of the first element that is `==` to `item`, counting from
/// 0, or `None`.
index_of = <A>(list : List(A), item : A) -> Option(Int) {
	go = (rest, i) {
		rest |>
			([]) { None }
			| ([x, ..xs]) {
				{ x == item } |> (True) { Some(i) } | (False) { go(xs, i + 1) }
			}
	}
	go(list, 0)
}

/// Puts the elements into groups by a key function, in a dict. Each group
/// keeps the elements in the order of the list.
///
/// ```i
/// list.group([1, 2, 3, 4], (x) { x % 2 })  // {1 -> [1, 3], 0 -> [2, 4]}
/// ```
group = <V, K, N>(list : List(V), by : (V) <- N -> K) -> Dict(K, List(V)) {
	dict.group(list, by)
}

/// Keeps the elements for which the function gives `True`, in order.
///
/// ```i
/// list.filter([1, 2, 3, 4], int.is_even)  // [2, 4]
/// ```
filter = <A, N>(list : List(A), keeping : (A) <- N -> Bool) -> List(A) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) {
				keeping(x) |>
					(True) { go(xs, [x, ..acc]) }
					| (False) { go(xs, acc) }
			}
	}
	go(list, [])
}

/// Calls a function on every element, and keeps the values of the `Some`
/// results.
///
/// ```i
/// list.filter_map(["1", "x", "3"], (s) { option.from_result(int.parse(s)) })  // [1, 3]
/// ```
filter_map = <A, B, N>(list : List(A), with : (A) <- N -> Option(B)) -> List(
	B,
) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) {
				with(x) |>
					(Some(y)) { go(xs, [y, ..acc]) }
					| (None) { go(xs, acc) }
			}
	}
	go(list, [])
}

/// Calls a function on every element, and gives the list of the results.
///
/// ```i
/// list.map([1, 2, 3], (x) { x * 2 })  // [2, 4, 6]
/// ```
map = <A, B, N>(list : List(A), with : (A) <- N -> B) -> List(B) {
	go = (rest, acc) {
		rest |> ([]) { reverse(acc) } | ([x, ..xs]) { go(xs, [with(x), ..acc]) }
	}
	go(list, [])
}

/// Calls a function on the elements of two lists at the same places. When
/// one list is longer, its other elements are not used.
///
/// ```i
/// list.map2([1, 2, 3], [4, 5], (x, y) { x + y })  // [5, 7]
/// ```
map2 = <A, B, C, N>(
	list1 : List(A),
	list2 : List(B),
	with : (A, B) <- N -> C
) -> List(C) {
	go = (xs, ys, acc) {
		(xs, ys) |>
			(([x, ..xs], [y, ..ys])) { go(xs, ys, [with(x, y), ..acc]) }
			| (_) { reverse(acc) }
	}
	go(list1, list2, [])
}

/// Maps the elements with an accumulator that goes from each element to the
/// next. Gives the last accumulator and the new list.
///
/// ```i
/// list.map_fold([1, 2, 3], 100, (acc, x) { (acc + x, x * 2) })
/// // (106, [2, 4, 6])
/// ```
map_fold = <A, Acc, B, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A) <- N -> (Acc, B)
) -> (Acc, List(B)) {
	go = (rest, acc, out) {
		rest |>
			([]) { (acc, reverse(out)) }
			| ([x, ..xs]) {
				(next, y) = with(acc, x)
				go(xs, next, [y, ..out])
			}
	}
	go(over, from, [])
}

/// Like `map`, and the function also gets the index of the element,
/// counting from 0.
index_map = <A, B, N>(list : List(A), with : (A, Int) <- N -> B) -> List(B) {
	go = (rest, i, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) { go(xs, i + 1, [with(x, i), ..acc]) }
	}
	go(list, 0, [])
}

/// Calls a function that gives a result on every element. Gives `Ok` with
/// all the values, or the first `Error`; after an `Error`, the function is
/// not called again.
///
/// ```i
/// list.try_map(["1", "2"], int.parse)  // Ok([1, 2])
/// ```
try_map = <A, B, F, N>(
	over : List(A),
	with : (A) <- N -> Result(B, F)
) -> Result(List(B), F) {
	go = (rest, acc) {
		rest |>
			([]) { Ok(reverse(acc)) }
			| ([x, ..xs]) {
				with(x) |>
					(Ok(y)) { go(xs, [y, ..acc]) }
					| (Error(error)) { Error(error) }
			}
	}
	go(over, [])
}

/// The list without its first `up_to` elements. When the list is shorter,
/// the empty list.
///
/// ```i
/// list.drop([1, 2, 3, 4], 2)  // [3, 4]
/// ```
drop = <A>(from : List(A), up_to : Int) -> List(A) {
	{ up_to <= 0 } |>
		(True) { from }
		| (False) { from |> ([]) { [] } | ([_, ..xs]) { drop(xs, up_to - 1) } }
}

/// The first `up_to` elements. When the list is shorter, the whole list.
///
/// ```i
/// list.take([1, 2, 3, 4], 2)  // [1, 2]
/// ```
take = <A>(from : List(A), up_to : Int) -> List(A) {
	go = (rest, n, acc) {
		{ n <= 0 } |>
			(True) { reverse(acc) }
			| (False) {
				rest |>
					([]) { reverse(acc) }
					| ([x, ..xs]) { go(xs, n - 1, [x, ..acc]) }
			}
	}
	go(from, up_to, [])
}

/// The empty list.
new = <A>() -> List(A) { [] }

/// A list of one element.
wrap = <A>(item : A) -> List(A) { [item] }

/// The elements of `first`, then the elements of `second`.
///
/// ```i
/// list.append([1, 2], [3])  // [1, 2, 3]
/// ```
append = <A>(first : List(A), second : List(A)) -> List(A) {
	reverse_onto(reverse(first), second)
}

/// Puts an element in front of a list.
prepend = <A>(to : List(A), this : A) -> List(A) { [this, ..to] }

/// The elements of all the lists, one list after the other.
///
/// ```i
/// list.concat([[1], [2, 3], []])  // [1, 2, 3]
/// ```
concat = <A>(lists : List(List(A))) -> List(A) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([xs, ..more]) { go(more, reverse_onto(xs, acc)) }
	}
	go(lists, [])
}

/// Maps every element to a list, and puts these lists together.
///
/// ```i
/// list.flat_map([1, 2], (x) { [x, x] })  // [1, 1, 2, 2]
/// ```
flat_map = <A, B, N>(over : List(A), with : (A) <- N -> List(B)) -> List(B) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) {
				go(
					xs,
					reverse_onto(with(x), acc),
				)
			}
	}
	go(over, [])
}

/// Combines the elements with an accumulator, from the first to the last.
///
/// ```i
/// list.fold([1, 2, 3], 0, (acc, x) { acc + x })  // 6
/// ```
fold = <A, Acc, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A) <- N -> Acc
) -> Acc {
	over |> ([]) { from } | ([x, ..xs]) { fold(xs, with(from, x), with) }
}

/// Combines the elements with an accumulator, from the last to the first.
///
/// ```i
/// list.fold_right([1, 2, 3], [], (acc, x) { [x, ..acc] })  // [1, 2, 3]
/// ```
fold_right = <A, Acc, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A) <- N -> Acc
) -> Acc { fold(reverse(over), from, with) }

/// Like `fold`, and the function also gets the index of the element.
index_fold = <A, Acc, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A, Int) <- N -> Acc
) -> Acc {
	go = (rest, acc, i) {
		rest |> ([]) { acc } | ([x, ..xs]) { go(xs, with(acc, x, i), i + 1) }
	}
	go(over, from, 0)
}

/// Like `fold`, but the function gives a result. The fold stops at the
/// first `Error`.
///
/// ```i
/// list.try_fold([1, 2, 3], 0, (acc, x) { Ok(acc + x) })  // Ok(6)
/// ```
try_fold = <A, Acc, F, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A) <- N -> Result(Acc, F)
) -> Result(Acc, F) {
	over |>
		([]) { Ok(from) }
		| ([x, ..xs]) {
			with(from, x) |>
				(Ok(next)) { try_fold(xs, next, with) }
				| (Error(error)) { Error(error) }
		}
}

/// Like `fold`, but the function says whether to go on (`Continue`) or to
/// stop (`Stop`).
///
/// ```i
/// list.fold_until([1, 2, 3, 4], 0, (acc, x) {
///   { x < 3 } |> { True -> list.Continue(acc + x)  False -> list.Stop(acc) }
/// })  // 3
/// ```
fold_until = <A, Acc, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A) <- N -> ContinueOrStop(Acc)
) -> Acc {
	over |>
		([]) { from }
		| ([x, ..xs]) {
			with(from, x) |>
				(Continue(next)) { fold_until(xs, next, with) }
				| (Stop(last)) { last }
		}
}

/// The first element for which the function gives `True`, or `None`.
///
/// ```i
/// list.find([1, 2, 3], (x) { x > 1 })  // Some(2)
/// ```
find = <A, N>(in : List(A), one_that : (A) <- N -> Bool) -> Option(A) {
	in |>
		([]) { None }
		| ([x, ..xs]) {
			one_that(x) |> (True) { Some(x) } | (False) { find(xs, one_that) }
		}
}

/// The first `Some` value that the function gives, or `None`.
find_map = <A, B, N>(in : List(A), with : (A) <- N -> Option(B)) -> Option(B) {
	in |>
		([]) { None }
		| ([x, ..xs]) {
			with(x) |> (Some(y)) { Some(y) } | (None) { find_map(xs, with) }
		}
}

/// `True` when the function gives `True` for every element. It stops at the
/// first `False`. `True` for the empty list.
all = <A, N>(in : List(A), satisfying : (A) <- N -> Bool) -> Bool {
	in |> ([]) { True } | ([x, ..xs]) { satisfying(x) && all(xs, satisfying) }
}

/// `True` when the function gives `True` for some element. It stops at the
/// first `True`. `False` for the empty list.
any = <A, N>(in : List(A), satisfying : (A) <- N -> Bool) -> Bool {
	in |> ([]) { False } | ([x, ..xs]) { satisfying(x) || any(xs, satisfying) }
}

/// Pairs of the elements of two lists at the same places. When one list is
/// longer, its other elements are not used.
///
/// ```i
/// list.zip([1, 2, 3], ["a", "b"])  // [(1, "a"), (2, "b")]
/// ```
zip = <A, B>(list : List(A), with : List(B)) -> List((A, B)) {
	go = (xs, ys, acc) {
		(xs, ys) |>
			(([x, ..xs], [y, ..ys])) { go(xs, ys, [(x, y), ..acc]) }
			| (_) { reverse(acc) }
	}
	go(list, with, [])
}

/// Like `zip`, but `Error(Nil)` when the lists have different lengths.
strict_zip = <A, B>(list : List(A), with : List(B)) -> Result(
	List((A, B)),
	Nil,
) {
	go = (xs, ys, acc) {
		(xs, ys) |>
			(([], [])) { Ok(reverse(acc)) }
			| (([x, ..xs], [y, ..ys])) { go(xs, ys, [(x, y), ..acc]) }
			| (_) { Error(Nil) }
	}
	go(list, with, [])
}

/// Takes a list of pairs apart into two lists.
///
/// ```i
/// list.unzip([(1, "a"), (2, "b")])  // ([1, 2], ["a", "b"])
/// ```
unzip = <A, B>(input : List((A, B))) -> (List(A), List(B)) {
	go = (rest, xs, ys) {
		rest |>
			([]) { (reverse(xs), reverse(ys)) }
			| ([(x, y), ..more]) { go(more, [x, ..xs], [y, ..ys]) }
	}
	go(input, [], [])
}

/// Puts `with` between every two elements.
///
/// ```i
/// list.intersperse([1, 2, 3], 0)  // [1, 0, 2, 0, 3]
/// ```
intersperse = <A>(list : List(A), with : A) -> List(A) {
	list |>
		([]) { [] }
		| ([x, ..xs]) {
			go = (rest, acc) {
				rest |>
					([]) { reverse(acc) }
					| ([y, ..ys]) { go(ys, [y, with, ..acc]) }
			}
			go(xs, [x])
		}
}

/// The list without the elements that are `==` to an earlier element. The
/// first of each is kept, in order. It takes O(n log n) steps, with a dict.
///
/// ```i
/// list.unique([1, 2, 1, 3, 2])  // [1, 2, 3]
/// ```
unique = <A>(list : List(A)) -> List(A) {
	go = (rest, seen, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) {
				dict.has(seen, x) |>
					(True) { go(xs, seen, acc) }
					| (False) { go(xs, dict.insert(seen, x, Nil), [x, ..acc]) }
			}
	}
	go(list, dict.new(), [])
}

/// Sorts the elements with a compare function. The sort is stable: equal
/// elements keep their order. It is a merge sort, which takes O(n log n)
/// steps, and which starts from the runs that are already in order.
///
/// ```i
/// list.sort([3, 1, 2], int.compare)  // [1, 2, 3]
/// ```
sort = <A, N>(list : List(A), by : (A, A) <- N -> Order) -> List(A) {
	list |> ([] | [_]) { list } | (_) { merge_all(runs(list, by, []), by) }
}

// Cuts a list into runs that are in order. A run that goes down strictly is
// turned round; one that goes down with equal elements is not taken as one
// run, so that the sort stays stable. The runs are given in order.
runs = <A, N>(
	list : List(A),
	by : (A, A) <- N -> Order,
	acc : List(List(A))
) -> List(List(A)) {
	list |>
		([]) { reverse(acc) }
		| ([x]) { reverse([[x], ..acc]) }
		| ([x, y, ..rest]) {
			by(x, y) |>
				(Greater) { descending(rest, y, [x], by, acc) }
				| (_) { ascending(rest, y, [x], by, acc) }
		}
}

// `run` is in reverse; `last` comes after it.
ascending = <A, N>(
	list : List(A),
	last : A,
	run : List(A),
	by : (A, A) <- N -> Order,
	acc : List(List(A))
) -> List(List(A)) {
	list |>
		([]) { reverse([reverse([last, ..run]), ..acc]) }
		| ([x, ..rest]) {
			by(last, x) |>
				(Greater) { runs(list, by, [reverse([last, ..run]), ..acc]) }
				| (_) { ascending(rest, x, [last, ..run], by, acc) }
		}
}

// `run` goes up (it is the strictly falling run, reversed); `last` comes
// before it.
descending = <A, N>(
	list : List(A),
	last : A,
	run : List(A),
	by : (A, A) <- N -> Order,
	acc : List(List(A))
) -> List(List(A)) {
	list |>
		([]) { reverse([[last, ..run], ..acc]) }
		| ([x, ..rest]) {
			by(last, x) |>
				(Greater) { descending(rest, x, [last, ..run], by, acc) }
				| (_) { runs(list, by, [[last, ..run], ..acc]) }
		}
}

merge_all = <A, N>(runs : List(List(A)), by : (A, A) <- N -> Order) -> List(A) {
	runs |>
		([]) { [] }
		| ([run]) { run }
		| (_) { merge_all(merge_pairs(runs, by, []), by) }
}

merge_pairs = <A, N>(
	runs : List(List(A)),
	by : (A, A) <- N -> Order,
	acc : List(List(A))
) -> List(List(A)) {
	runs |>
		([]) { reverse(acc) }
		| ([run]) { reverse([run, ..acc]) }
		| ([a, b, ..rest]) {
			merge_pairs(rest, by, [merge(a, b, by, []), ..acc])
		}
}

// Merges two sorted lists. On a tie, the element of `xs` comes first.
merge = <A, N>(
	xs : List(A),
	ys : List(A),
	by : (A, A) <- N -> Order,
	acc : List(A)
) -> List(A) {
	xs |>
		([]) { reverse_onto(acc, ys) }
		| ([x, ..xs2]) {
			ys |>
				([]) { reverse_onto(acc, xs) }
				| ([y, ..ys2]) {
					by(x, y) |>
						(Greater) { merge(xs, ys2, by, [y, ..acc]) }
						| (_) { merge(xs2, ys, by, [x, ..acc]) }
				}
		}
}

/// The numbers from `from` to `to`, with both ends. It counts down when
/// `to` is less than `from`.
///
/// ```i
/// list.range(1, 4)  // [1, 2, 3, 4]
/// list.range(2, 0)  // [2, 1, 0]
/// ```
range = (from : Int, to : Int) -> List(Int) {
	go = (i, acc) {
		{ i == from } |>
			(True) { [i, ..acc] }
			| (False) {
				{ i < from } |>
					(True) { go(i + 1, [i, ..acc]) }
					| (False) { go(i - 1, [i, ..acc]) }
			}
	}
	go(to, [])
}

/// A list of `times` copies of `item`.
///
/// ```i
/// list.repeat("a", 3)  // ["a", "a", "a"]
/// ```
repeat = <A>(item : A, times : Int) -> List(A) {
	go = (n, acc) {
		{ n <= 0 } |> (True) { acc } | (False) { go(n - 1, [item, ..acc]) }
	}
	go(times, [])
}

/// The first `at` elements, and the rest.
///
/// ```i
/// list.split([1, 2, 3, 4], 1)  // ([1], [2, 3, 4])
/// ```
split = <A>(list : List(A), at : Int) -> (List(A), List(A)) {
	go = (rest, n, acc) {
		{ n <= 0 } |>
			(True) { (reverse(acc), rest) }
			| (False) {
				rest |>
					([]) { (reverse(acc), []) }
					| ([x, ..xs]) { go(xs, n - 1, [x, ..acc]) }
			}
	}
	go(list, at, [])
}

/// The first elements for which the function gives `True`, and the rest.
split_while = <A, N>(list : List(A), satisfying : (A) <- N -> Bool) -> (
	List(A),
	List(A),
) {
	go = (rest, acc) {
		rest |>
			([]) { (reverse(acc), []) }
			| ([x, ..xs]) {
				satisfying(x) |>
					(True) { go(xs, [x, ..acc]) }
					| (False) { (reverse(acc), rest) }
			}
	}
	go(list, [])
}

/// The value of the first pair whose key is `==` to `find`, in a list of
/// pairs, or `None`.
///
/// ```i
/// list.key_find([("a", 1), ("b", 2)], "b")  // Some(2)
/// ```
key_find = <K, V>(in : List((K, V)), find : K) -> Option(V) {
	in |>
		([]) { None }
		| ([(k, v), ..rest]) {
			{ k == find } |>
				(True) { Some(v) }
				| (False) { key_find(rest, find) }
		}
}

/// The values of all the pairs whose key is `==` to `find`, in order.
key_filter = <K, V>(in : List((K, V)), find : K) -> List(V) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([(k, v), ..xs]) {
				{ k == find } |>
					(True) { go(xs, [v, ..acc]) }
					| (False) { go(xs, acc) }
			}
	}
	go(in, [])
}

/// The value of the first pair whose key is `==` to `key`, and the list
/// without that pair, or `None`.
///
/// ```i
/// list.key_pop([("a", 1), ("b", 2)], "a")  // Some((1, [("b", 2)]))
/// ```
key_pop = <K, V>(list : List((K, V)), key : K) -> Option((V, List((K, V)))) {
	go = (rest, acc) {
		rest |>
			([]) { None }
			| ([(k, v), ..xs]) {
				{ k == key } |>
					(True) { Some((v, reverse_onto(acc, xs))) }
					| (False) { go(xs, [(k, v), ..acc]) }
			}
	}
	go(list, [])
}

/// Sets the value of the first pair whose key is `==` to `key`, or adds the
/// pair at the end when there is none.
key_set = <K, V>(list : List((K, V)), key : K, value : V) -> List((K, V)) {
	go = (rest, acc) {
		rest |>
			([]) { reverse([(key, value), ..acc]) }
			| ([(k, v), ..xs]) {
				{ k == key } |>
					(True) { reverse_onto(acc, [(key, value), ..xs]) }
					| (False) { go(xs, [(k, v), ..acc]) }
			}
	}
	go(list, [])
}

/// Calls a function on every element, in order, for its effects.
each = <A, B, N>(list : List(A), with : (A) <- N -> B) -> Nil {
	list |>
		([]) { Nil }
		| ([x, ..xs]) {
			with(x)
			each(xs, with)
		}
}

/// Calls a function that gives a result on every element, in order, and
/// stops at the first `Error`.
try_each = <A, B, F, N>(
	over : List(A),
	with : (A) <- N -> Result(B, F)
) -> Result(Nil, F) {
	over |>
		([]) { Ok(Nil) }
		| ([x, ..xs]) {
			with(x) |>
				(Ok(_)) { try_each(xs, with) }
				| (Error(error)) { Error(error) }
		}
}

/// The elements for which the function gives `True`, and the others, each
/// in order.
///
/// ```i
/// list.partition([1, 2, 3, 4], int.is_odd)  // ([1, 3], [2, 4])
/// ```
partition = <A, N>(list : List(A), with : (A) <- N -> Bool) -> (
	List(A),
	List(A),
) {
	go = (rest, yes, no) {
		rest |>
			([]) { (reverse(yes), reverse(no)) }
			| ([x, ..xs]) {
				with(x) |>
					(True) { go(xs, [x, ..yes], no) }
					| (False) { go(xs, yes, [x, ..no]) }
			}
	}
	go(list, [], [])
}

/// Every order of the elements. Elements are told apart by their place, so
/// equal elements give equal permutations more than once. The number of
/// permutations is n!, so this is only for short lists.
///
/// ```i
/// list.permutations([1, 2, 3])
/// // [[1, 2, 3], [1, 3, 2], [2, 1, 3], [2, 3, 1], [3, 1, 2], [3, 2, 1]]
/// ```
permutations = <A>(list : List(A)) -> List(List(A)) {
	list |>
		([]) { [[]] }
		| (_) {
			// Each element in turn, with the others in order.
			picks = (before, after, acc) {
				after |>
					([]) { reverse(acc) }
					| ([x, ..xs]) {
						picks(
							[x, ..before],
							xs,
							[(x, reverse_onto(before, xs)), ..acc],
						)
					}
			}
			flat_map(picks([], list, []), (pick) {
				(x, others) = pick
				map(permutations(others), (p) { [x, ..p] })
			})
		}
}

/// Every run of `by` elements that come one after the other, in order.
///
/// ```i
/// list.window([1, 2, 3, 4], 2)  // [[1, 2], [2, 3], [3, 4]]
/// ```
window = <A>(list : List(A), by : Int) -> List(List(A)) {
	{ by <= 0 } |>
		(True) { [] }
		| (False) {
			go = (rest, acc) {
				w = take(rest, by)
				{ length(w) == by } |>
					(True) {
						rest |>
							([_, ..xs]) { go(xs, [w, ..acc]) }
							| ([]) { reverse(acc) }
					}
					| (False) { reverse(acc) }
			}
			go(list, [])
		}
}

/// Every pair of elements that come one after the other.
///
/// ```i
/// list.window_by_2([1, 2, 3])  // [(1, 2), (2, 3)]
/// ```
window_by_2 = <A>(list : List(A)) -> List((A, A)) {
	list |> ([]) { [] } | ([_, ..xs]) { zip(list, xs) }
}

/// The list without its first elements for which the function gives
/// `True`.
drop_while = <A, N>(in : List(A), satisfying : (A) <- N -> Bool) -> List(A) {
	in |>
		([]) { [] }
		| ([x, ..xs]) {
			satisfying(x) |>
				(True) { drop_while(xs, satisfying) }
				| (False) { in }
		}
}

/// The first elements for which the function gives `True`.
take_while = <A, N>(in : List(A), satisfying : (A) <- N -> Bool) -> List(A) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) {
				satisfying(x) |>
					(True) { go(xs, [x, ..acc]) }
					| (False) { reverse(acc) }
			}
	}
	go(in, [])
}

/// Cuts the list into runs of elements, one after the other, for which the
/// function gives the same key.
///
/// ```i
/// list.chunk([1, 2, 2, 3, 4, 4, 6], (x) { x % 2 })
/// // [[1], [2, 2], [3], [4, 4, 6]]
/// ```
chunk = <A, K, N>(in : List(A), by : (A) <- N -> K) -> List(List(A)) {
	in |>
		([]) { [] }
		| ([x, ..xs]) {
			go = (rest, key, current, acc) {
				rest |>
					([]) { reverse([reverse(current), ..acc]) }
					| ([y, ..ys]) {
						k = by(y)
						{ k == key } |>
							(True) { go(ys, key, [y, ..current], acc) }
							| (False) {
								go(ys, k, [y], [reverse(current), ..acc])
							}
					}
			}
			go(xs, by(x), [x], [])
		}
}

/// Cuts the list into lists of `into` elements. The last list may be
/// shorter. When `into` is 0 or less, the empty list.
///
/// ```i
/// list.sized_chunk([1, 2, 3, 4, 5], 2)  // [[1, 2], [3, 4], [5]]
/// ```
sized_chunk = <A>(in : List(A), into : Int) -> List(List(A)) {
	{ into <= 0 } |>
		(True) { [] }
		| (False) {
			go = (rest, n, current, acc) {
				rest |>
					([]) {
						current |>
							([]) { reverse(acc) }
							| (_) { reverse([reverse(current), ..acc]) }
					}
					| ([x, ..xs]) {
						{ n == 1 } |>
							(True) {
								go(
									xs,
									into,
									[],
									[reverse([x, ..current]), ..acc],
								)
							}
							| (False) { go(xs, n - 1, [x, ..current], acc) }
					}
			}
			go(in, into, [], [])
		}
}

/// Combines the elements with a function, from the first to the last, with
/// the first element as the start. `None` for the empty list.
///
/// ```i
/// list.reduce([1, 2, 3], (a, b) { a + b })  // Some(6)
/// ```
reduce = <A, N>(over : List(A), with : (A, A) <- N -> A) -> Option(A) {
	over |> ([]) { None } | ([x, ..xs]) { Some(fold(xs, x, with)) }
}

/// Like `fold`, but gives every accumulator after each element.
///
/// ```i
/// list.scan([1, 2, 3], 0, (acc, x) { acc + x })  // [1, 3, 6]
/// ```
scan = <A, Acc, N>(
	over : List(A),
	from : Acc,
	with : (Acc, A) <- N -> Acc
) -> List(Acc) {
	go = (rest, acc, out) {
		rest |>
			([]) { reverse(out) }
			| ([x, ..xs]) {
				next = with(acc, x)
				go(xs, next, [next, ..out])
			}
	}
	go(over, from, [])
}

/// Every way to choose `by` elements, keeping their order.
///
/// ```i
/// list.combinations([1, 2, 3], 2)  // [[1, 2], [1, 3], [2, 3]]
/// ```
combinations = <A>(items : List(A), by : Int) -> List(List(A)) {
	{ by <= 0 } |>
		(True) { [[]] }
		| (False) {
			items |>
				([]) { [] }
				| ([x, ..xs]) {
					append(
						map(combinations(xs, by - 1), (c) { [x, ..c] }),
						combinations(xs, by),
					)
				}
		}
}

/// Every pair of two elements, keeping their order.
///
/// ```i
/// list.combination_pairs([1, 2, 3])  // [(1, 2), (1, 3), (2, 3)]
/// ```
combination_pairs = <A>(items : List(A)) -> List((A, A)) {
	go = (rest, acc) {
		rest |>
			([]) { reverse(acc) }
			| ([x, ..xs]) {
				go(
					xs,
					reverse_onto(map(xs, (y) { (x, y) }), acc),
				)
			}
	}
	go(items, [])
}

/// The first elements of all the lists, then the second elements, and so on.
///
/// ```i
/// list.interleave([[1, 2], [101, 102], [201]])  // [1, 101, 201, 2, 102]
/// ```
interleave = <A>(list : List(List(A))) -> List(A) { concat(transpose(list)) }

/// Turns rows into columns. Shorter lists leave gaps, which are taken out.
///
/// ```i
/// list.transpose([[1, 2, 3], [101, 102, 103]])  // [[1, 101], [2, 102], [3, 103]]
/// ```
transpose = <A>(list_of_lists : List(List(A))) -> List(List(A)) {
	go = (rows, acc) {
		nonempty = filter(rows, (row) { row != [] })
		nonempty |>
			([]) { reverse(acc) }
			| (_) {
				heads = filter_map(nonempty, first)
				tails = map(nonempty, (row) { drop(row, 1) })
				go(tails, [heads, ..acc])
			}
	}
	go(list_of_lists, [])
}

/// The greatest element by a compare function, or `None` for the empty
/// list. Of equal greatest elements, the first is given.
max = <A, N>(over : List(A), with : (A, A) <- N -> Order) -> Option(A) {
	over |>
		([]) { None }
		| ([x, ..xs]) {
			Some(
				fold(xs, x, (best, y) {
					with(y, best) |> (Greater) { y } | (_) { best }
				}),
			)
		}
}

/// The smallest element by a compare function, or `None` for the empty
/// list. Of equal smallest elements, the first is given.
min = <A, N>(over : List(A), with : (A, A) <- N -> Order) -> Option(A) {
	over |>
		([]) { None }
		| ([x, ..xs]) {
			Some(
				fold(xs, x, (best, y) {
					with(y, best) |> (Less) { y } | (_) { best }
				}),
			)
		}
}
