(
	testlib = (test) = std/test
	i/core
	i/fail
	i/control
	result = std/result
	dict = std/dict
	float = std/float
	int = std/int
	list = (Continue, Stop) = std/list
	pair = std/pair
	string = std/string
	order = (Less, Equal, Greater) = std/order
	option = (Some, None) = std/option
)

// Wasm tail calls must handle the same depth as the Erlang baseline.
recursion_test_cycles = 1_000_000

test("length_empty_test", () {
	zero = 0
	assert(list.length([]) == zero)
	Nil
})

test("length_one_test", () {
	assert(list.length([1]) == 1)
	Nil
})

test("length_two_test", () {
	assert(list.length([1, 1]) == 2)
	Nil
})

test("length_three_test", () {
	assert(list.length([1, 1, 1]) == 3)
	Nil
})

test("length_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend) |> list.length()
	Nil
})

test("count_empty_test", () {
	assert(list.count([], int.is_odd) == 0)
	Nil
})

test("count_no_matches_test", () {
	assert(list.count([2, 4, 6], int.is_odd) == 0)
	Nil
})

test("count_some_matches_test", () {
	assert(list.count([1, 2, 3, 4, 5], int.is_odd) == 3)
	Nil
})

test("count_strings_test", () {
	assert(
		list.count(["a", "list", "with", "some", "string", "values"], (a) {
			string.length(a) > 4
		}) ==
			2
	)
	Nil
})

test("reverse_empty_test", () {
	assert(list.reverse([]) == [])
	Nil
})

test("reverse_one_test", () {
	assert(list.reverse([1]) == [1])
	Nil
})

test("reverse_two_test", () {
	assert(list.reverse([1, 2]) == [2, 1])
	Nil
})

test("reverse_five_test", () {
	assert(list.reverse([1, 2, 3, 4, 5]) == [5, 4, 3, 2, 1])
	Nil
})

test("reverse_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend) |> list.reverse
	Nil
})

test("is_empty_empty_test", () {
	assert(list.is_empty([]))
	Nil
})

test("is_empty_non_empty_test", () {
	assert(core.not(list.is_empty([1])))
	Nil
})

test("contains_present_test", () {
	assert(list.contains([0, 4, 5, 1], 1))
	Nil
})

test("contains_absent_test", () {
	assert(core.not(list.contains([0, 4, 5, 7], 1)))
	Nil
})

test("contains_empty_test", () {
	assert(core.not(list.contains([], 1)))
	Nil
})

test("contains_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.contains(1)
	Nil
})

test("first_ok_test", () {
	assert(list.first([0, 4, 5, 7]) == Some(0))
	Nil
})

test("first_error_test", () {
	assert(list.first([]) == None)
	Nil
})

test("rest_ok_test", () {
	assert(list.rest([0, 4, 5, 7]) == Some([4, 5, 7]))
	Nil
})

test("rest_single_test", () {
	assert(list.rest([0]) == Some([]))
	Nil
})

test("rest_error_test", () {
	assert(list.rest([]) == None)
	Nil
})

test("group_test", () {
	assert(list.group([Ok(10), Error("Wrong"), Ok(200), Ok(1)], (i) {
		i |> {
			Ok(_)    -> "Successful"
			Error(_) -> "Failed"
		}
	}) == {
		dict.new()
			|> dict.insert("Failed", [Error("Wrong")])
			|> dict.insert("Successful", [Ok(10), Ok(200), Ok(1)])
	})
	Nil
})

test("group_modulo_test", () {
	assert(list.group([1, 2, 3, 4, 5], (i) { i - i / 3 * 3 }) == {
		dict.new()
			|> dict.insert(0, [3])
			|> dict.insert(1, [1, 4])
			|> dict.insert(2, [2, 5])
	})
	Nil
})

test("filter_empty_test", () {
	assert(list.filter([], (_) { True }) == [])
	Nil
})

test("filter_all_true_test", () {
	assert(list.filter([0, 4, 5, 7, 3], (_) { True }) == [0, 4, 5, 7, 3])
	Nil
})

test("filter_greater_than_test", () {
	assert(list.filter([0, 4, 5, 7, 3], (x) { x > 4 }) == [5, 7])
	Nil
})

test("filter_less_than_test", () {
	assert(list.filter([0, 4, 5, 7, 3], (x) { x < 4 }) == [0, 3])
	Nil
})

test("filter_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend) |> list.filter((x) {
		x == -1
	})
	Nil
})

test("filter_map_ok_test", () {
	assert(list.filter_map([2, 4, 6, 1], (x) { Some(x + 1) }) == [3, 5, 7, 2])
	Nil
})

test("filter_map_error_test", () {
	assert(list.filter_map([2, 4, 6, 1], (_) { None }) == [])
	Nil
})

test("filter_map_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.filter_map((x) {
		Some(x + 1)
	})
	Nil
})

test("map_empty_test", () {
	assert(list.map([], (x) { x * 2 }) == [])
	Nil
})

test("map_double_test", () {
	assert(list.map([0, 4, 5, 7, 3], (x) { x * 2 }) == [0, 8, 10, 14, 6])
	Nil
})

test("map_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.map((x) { x })
	Nil
})

test("map2_empty_second_test", () {
	assert(list.map2([1, 2, 3], [], int.add) == [])
	Nil
})

test("map2_empty_first_test", () {
	assert(list.map2([], [1, 2, 3], int.add) == [])
	Nil
})

test("map2_both_empty_test", () {
	assert(list.map2([], [], int.add) == [])
	Nil
})

test("map2_shorter_second_test", () {
	assert(list.map2([1, 2, 3], [4, 5], int.add) == [5, 7])
	Nil
})

test("map2_equal_length_test", () {
	assert(list.map2([1, 2, 3], [4, 5, 6], int.add) == [5, 7, 9])
	Nil
})

test("map2_pair_test", () {
	assert(list.map2([1, 2, 3], ["1", "2"], pair.new) == [(1, "1"), (2, "2")])
	Nil
})

test("map2_tco_test", () {
	// TCO test
	xs = list.repeat(0, recursion_test_cycles)
	list.map2(xs, xs, int.add)
	Nil
})

test("map_fold_test", () {
	assert(
		list.map_fold([1, 2, 3, 4], 0, (acc, i) { (acc + i, i * 2) }) ==
			(10, [2, 4, 6, 8])
	)
	Nil
})

test("map_fold_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.map_fold(0, (acc, i) {
		(acc + i, i + 1)
	})
	Nil
})

test("try_map_ok_test", () {
	fun = (x) {
		{ x == 6 || x == 5 || x == 4 } |> {
			True  -> Ok(x * 2)
			False -> Error(x)
		}
	}

	assert(list.try_map([5, 6, 5, 6], fun) == Ok([10, 12, 10, 12]))
	Nil
})

test("try_map_error_test", () {
	fun = (x) {
		{ x == 6 || x == 5 || x == 4 } |> {
			True  -> Ok(x * 2)
			False -> Error(x)
		}
	}

	assert(list.try_map([4, 6, 5, 7, 3], fun) == Error(7))
	Nil
})

test("try_map_tco_test", () {
	fun = (x) {
		{ x == 6 || x == 5 || x == 4 } |> {
			True  -> Ok(x * 2)
			False -> Error(x)
		}
	}

	// TCO test
	list.repeat(6, recursion_test_cycles) |> list.try_map(fun)
	Nil
})

test("drop_empty_test", () {
	assert(list.drop([], 5) == [])
	Nil
})

test("drop_five_test", () {
	assert(list.drop([1, 2, 3, 4, 5, 6, 7, 8], 5) == [6, 7, 8])
	Nil
})

test("drop_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.drop(recursion_test_cycles)
	Nil
})

test("take_empty_test", () {
	assert(list.take([], 5) == [])
	Nil
})

test("take_five_test", () {
	assert(list.take([1, 2, 3, 4, 5, 6, 7, 8], 5) == [1, 2, 3, 4, 5])
	Nil
})

test("take_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.take(recursion_test_cycles)
	Nil
})

test("new_test", () {
	assert(list.new() == [])
	Nil
})

test("wrap_empty_test", () {
	assert(list.wrap([]) == [[]])
	Nil
})

test("wrap_nested_empty_test", () {
	assert(list.wrap([[]]) == [[[]]])
	Nil
})

test("wrap_nil_test", () {
	assert(list.wrap(Nil) == [Nil])
	Nil
})

test("wrap_int_test", () {
	assert(list.wrap(1) == [1])
	Nil
})

test("wrap_list_test", () {
	assert(list.wrap([1, 2]) == [[1, 2]])
	Nil
})

test("wrap_nested_list_test", () {
	assert(list.wrap([[1, 2, 3]]) == [[[1, 2, 3]]])
	Nil
})

test("append_basic_test", () {
	assert(list.append([1], [2, 3]) == [1, 2, 3])
	Nil
})

test("append_empty_second_test", () {
	assert(list.append([1, 2], []) == [1, 2])
	Nil
})

test("append_empty_first_test", () {
	assert(list.append([], [1, 2]) == [1, 2])
	Nil
})

test("append_equal_length_test", () {
	assert(list.append([1, 2], [3, 4]) == [1, 2, 3, 4])
	Nil
})

test("append_longer_first_test", () {
	assert(list.append([1, 2, 3], []) == [1, 2, 3])
	Nil
})

test("append_single_second_test", () {
	assert(list.append([1, 2, 3], [4]) == [1, 2, 3, 4])
	Nil
})

test("append_four_one_test", () {
	assert(list.append([1, 2, 3, 4], [5]) == [1, 2, 3, 4, 5])
	Nil
})

test("append_both_empty_test", () {
	assert(list.append([], []) == [])
	Nil
})

test("append_tco_test", () {
	xs = list.range(1, big)
	assert(list.length(list.append(xs, xs)) == 2 * big)
})

test("flatten_empty_test", () {
	assert(list.concat([]) == [])
	Nil
})

test("flatten_single_empty_test", () {
	assert(list.concat([[]]) == [])
	Nil
})

test("flatten_multiple_empty_test", () {
	assert(list.concat([[], [], []]) == [])
	Nil
})

test("flatten_mixed_test", () {
	assert(list.concat([[1, 2], [], [3, 4]]) == [1, 2, 3, 4])
	Nil
})

test("flat_map_test", () {
	assert(
		list.flat_map([1, 10, 20], (x) { [x, x + 1] }) == [1, 2, 10, 11, 20, 21]
	)
	Nil
})

test("fold_test", () {
	assert(list.fold([1, 2, 3], [], (acc, x) { [x, ..acc] }) == [3, 2, 1])
	Nil
})

test("fold_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend)
		|> list.fold([], (acc, x) { [x, ..acc] })
	Nil
})

test("fold_right_test", () {
	assert(list.fold_right([1, 2, 3], [], (acc, x) { [x, ..acc] }) == [1, 2, 3])
	Nil
})

test("index_fold_test", () {
	assert(
		list.index_fold(["a", "b", "c"], [], (acc, x, i) { [(i, x), ..acc] }) ==
			[(2, "c"), (1, "b"), (0, "a")]
	)
})

test("index_fold_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend)
		|> list.index_fold([], (acc, i, ix) { [(ix, i), ..acc] })
	Nil
})

test("fold_until_test", () {
	assert(list.fold_until([1, 2, 3, 4], 0, (acc, n) {
		{ n < 4 } |> {
			True  -> list.Continue(acc + n)
			False -> list.Stop(acc)
		}
	}) == 6)
	Nil
})

test("fold_until_tco_test", () {
	r = list.fold_until(list.range(1, big), 0, (acc, x) {
		{ x > 10 } |> {
			True  -> Stop(acc)
			False -> Continue(acc + x)
		}
	})
	assert(r == 55)
	assert(
		list.fold_until(list.repeat(1, big), 0, (acc, x) {
			Continue(acc + x)
		}) ==
			big
	)
})

test("try_fold_ok_test", () {
	assert(list.try_fold([1, 2, 3], 0, (acc, i) {
		{ i < 4 } |> {
			True  -> Ok(acc + i)
			False -> Error(Nil)
		}
	}) == Ok(6))
	Nil
})

test("try_fold_error_test", () {
	assert(list.try_fold([1, 2, 3], 0, (acc, i) {
		{ i < 3 } |> {
			True  -> Ok(acc + i)
			False -> Error(Nil)
		}
	}) == Error(Nil))
	Nil
})

test("try_fold_tco_test", () {
	assert(
		list.try_fold(list.repeat(1, big), 0, (acc, x) { Ok(acc + x) }) ==
			Ok(big)
	)
})

test("find_map_found_test", () {
	f = (x) {
		x |> {
			2 -> Some(4)
			_ -> None
		}
	}

	assert(list.find_map([1, 2, 3], f) == Some(4))
	Nil
})

test("find_map_found_different_order_test", () {
	f = (x) {
		x |> {
			2 -> Some(4)
			_ -> None
		}
	}

	assert(list.find_map([1, 3, 2], f) == Some(4))
	Nil
})

test("find_map_not_found_test", () {
	f = (x) {
		x |> {
			2 -> Some(4)
			_ -> None
		}
	}

	assert(list.find_map([1, 3], f) == None)
	Nil
})

test("find_map_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend)
		|> list.find_map((x) {
			{ x == recursion_test_cycles } |> {
				True  -> Some(recursion_test_cycles)
				False -> None
			}
		})
	Nil
})

test("find_found_test", () {
	is_two = (x) { x == 2 }

	assert(list.find([1, 2, 3], is_two) == Some(2))
	Nil
})

test("find_found_different_order_test", () {
	is_two = (x) { x == 2 }

	assert(list.find([1, 3, 2], is_two) == Some(2))
	Nil
})

test("find_not_found_test", () {
	is_two = (x) { x == 2 }

	assert(list.find([1, 3], is_two) == None)
	Nil
})

test("find_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend) |> list.find((x) {
		x == recursion_test_cycles
	})
	Nil
})

test("all_true_test", () {
	assert(list.all([1, 2, 3, 4, 5], (x) { x > 0 }))
	Nil
})

test("all_false_test", () {
	assert(core.not(list.all([1, 2, 3, 4, 5], (x) { x < 0 })))
	Nil
})

test("all_empty_test", () {
	assert(list.all([], (_) { False }))
	Nil
})

test("all_short_circuit_test", () {
	[1, 2, 3] |> list.all((x) {
		x |> {
			1 -> True
			2 -> False
			// Crash if no short-circuit
			_ -> panic("unreachable")
		}
	})
	Nil
})

test("all_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles) |> list.all((x) {
		x |> {
			0 -> True
			_ -> False
		}
	})
	Nil
})

test("any_true_test", () {
	assert(list.any([1, 2, 3, 4, 5], (x) { x == 2 }))
	Nil
})

test("any_false_test", () {
	assert(core.not(list.any([1, 2, 3, 4, 5], (x) { x < 0 })))
	Nil
})

test("any_empty_test", () {
	assert(core.not(list.any([], (_) { False })))
	Nil
})

test("any_short_circuit_test", () {
	[1, 2, 3] |> list.any((x) {
		x |> {
			1 -> False
			2 -> True
			// Crash if no short-circuit
			_ -> panic("unreachable")
		}
	})
	Nil
})

test("any_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend) |> list.any((x) {
		x == recursion_test_cycles
	})
	Nil
})

test("zip_empty_second_test", () {
	assert(list.zip([], [1, 2, 3]) == [])
	Nil
})

test("zip_empty_first_test", () {
	assert(list.zip([1, 2], []) == [])
	Nil
})

test("zip_equal_length_test", () {
	assert(list.zip([1, 2, 3], [4, 5, 6]) == [(1, 4), (2, 5), (3, 6)])
	Nil
})

test("zip_shorter_second_test", () {
	assert(list.zip([5, 6], [1, 2, 3]) == [(5, 1), (6, 2)])
	Nil
})

test("zip_shorter_first_test", () {
	assert(list.zip([5, 6, 7], [1, 2]) == [(5, 1), (6, 2)])
	Nil
})

test("zip_tco_test", () {
	// TCO test
	recursion_test_cycles_list = int.range(
		recursion_test_cycles
		-1
		[]
		list.prepend
	)
	list.zip(recursion_test_cycles_list, recursion_test_cycles_list)
	Nil
})

test("strict_zip_empty_second_error_test", () {
	assert(list.strict_zip([], [1, 2, 3]) == Error(Nil))
	Nil
})

test("strict_zip_empty_first_error_test", () {
	assert(list.strict_zip([1, 2], []) == Error(Nil))
	Nil
})

test("strict_zip_equal_length_ok_test", () {
	assert(
		list.strict_zip([1, 2, 3], [4, 5, 6]) == Ok([(1, 4), (2, 5), (3, 6)])
	)
	Nil
})

test("strict_zip_shorter_second_error_test", () {
	assert(list.strict_zip([5, 6], [1, 2, 3]) == Error(Nil))
	Nil
})

test("strict_zip_shorter_first_error_test", () {
	assert(list.strict_zip([5, 6, 7], [1, 2]) == Error(Nil))
	Nil
})

test("unzip_test", () {
	assert(list.unzip([(1, 2), (3, 4)]) == ([1, 3], [2, 4]))
	Nil
})

test("unzip_empty_test", () {
	assert(list.unzip([]) == ([], []))
	Nil
})

test("unzip_tco_test", () {
	// TCO test
	recursion_test_cycles_list = int.range(
		recursion_test_cycles
		-1
		[]
		list.prepend
	)
	list.zip(recursion_test_cycles_list, recursion_test_cycles_list)
		|> list.unzip()
	Nil
})

test("intersperse_test", () {
	assert(list.intersperse([1, 2, 3], 4) == [1, 4, 2, 4, 3])
	Nil
})

test("intersperse_empty_test", () {
	assert(list.intersperse([], 2) == [])
	Nil
})

test("intersperse_tco_test", () {
	assert(list.length(list.intersperse(list.repeat(1, big), 0)) == 2 * big - 1)
})

test("unique_duplicates_test", () {
	assert(list.unique([1, 1, 2, 3, 4, 4, 4, 5, 6]) == [1, 2, 3, 4, 5, 6])
	Nil
})

test("unique_scattered_test", () {
	assert(
		list.unique([7, 1, 45, 6, 2, 47, 2, 7, 5]) == [7, 1, 45, 6, 2, 47, 5]
	)
	Nil
})

test("unique_no_duplicates_test", () {
	assert(list.unique([3, 4, 5]) == [3, 4, 5])
	Nil
})

test("unique_empty_test", () {
	assert(list.unique([]) == [])
	Nil
})

test("unique_tco_test", () {
	xs = list.map(list.range(1, big), (i) { i % 1000 })
	assert(list.unique(xs) == list.append(list.range(1, 999), [0]))
})

test("sort_empty_test", () {
	assert(list.sort([], int.compare) == [])
	Nil
})

test("sort_single_test", () {
	assert(list.sort([1], int.compare) == [1])
	Nil
})

test("sort_two_test", () {
	assert(list.sort([2, 1], int.compare) == [1, 2])
	Nil
})

test("sort_three_test", () {
	assert(list.sort([3, 1, 2], int.compare) == [1, 2, 3])
	Nil
})

test("sort_five_test", () {
	assert(list.sort([4, 3, 6, 5, 4], int.compare) == [3, 4, 4, 5, 6])
	Nil
})

test("sort_six_test", () {
	assert(list.sort([4, 3, 6, 5, 4, 1], int.compare) == [1, 3, 4, 4, 5, 6])
	Nil
})

test("sort_float_test", () {
	assert(
		list.sort([4.1, 3.1, 6.1, 5.1, 4.1], float.compare) ==
			[3.1, 4.1, 4.1, 5.1, 6.1]
	)
	Nil
})

test("sort_empty_2_test", () {
	assert(list.sort([], int.compare) == [])
	Nil
})

test("sort_stability_test", () {
	// Stability tests
	sorted_cards = [
		(1, 1)
		(2, 1)
		(3, 1)
		(4, 1)
		(1, 2)
		(2, 2)
		(3, 2)
		(4, 2)
		(1, 3)
		(2, 3)
		(3, 3)
		(4, 3)
		(1, 4)
		(2, 4)
		(3, 4)
		(4, 4)
	]
	shuffled_cards = [
		(3, 2)
		(1, 4)
		(2, 1)
		(3, 3)
		(4, 1)
		(3, 4)
		(1, 2)
		(4, 4)
		(3, 1)
		(1, 1)
		(2, 2)
		(2, 4)
		(4, 2)
		(4, 3)
		(1, 3)
		(2, 3)
	]
	assert(
		{
			shuffled_cards
				|> list.sort((a, b) { int.compare(a.0, b.0) })
				|> list.sort((a, b) { int.compare(a.1, b.1) })
		} ==
			sorted_cards
	)
	Nil
})

test("sort_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend)
		|> list.reverse
		|> list.sort(int.compare)
	Nil
})

test("index_map_test", () {
	assert(
		list.index_map([3, 4, 5], (x, i) { (i, x) }) == [(0, 3), (1, 4), (2, 5)]
	)
	Nil
})

test("index_map_strings_test", () {
	f = (x, i) { (x, i) }
	assert(list.index_map(["a", "b"], f) == [("a", 0), ("b", 1)])
	Nil
})

test("index_map_three_strings_test", () {
	f = (x, i) { (x, i) }
	assert(list.index_map(["a", "b", "c"], f) == [("a", 0), ("b", 1), ("c", 2)])
	Nil
})

test("repeat_negative_test", () {
	assert(list.repeat(1, -10) == [])
	Nil
})

test("repeat_zero_test", () {
	assert(list.repeat(1, 0) == [])
	Nil
})

test("repeat_three_test", () {
	assert(list.repeat(2, 3) == [2, 2, 2])
	Nil
})

test("repeat_five_test", () {
	assert(list.repeat("x", 5) == ["x", "x", "x", "x", "x"])
	Nil
})

test("repeat_tco_test", () {
	// TCO test
	list.repeat(0, recursion_test_cycles)
	Nil
})

test("split_zero_test", () {
	assert(list.split([], 0) == ([], []))
	Nil
})

test("split_at_zero_test", () {
	assert(list.split([0, 1, 2, 3, 4], 0) == ([], [0, 1, 2, 3, 4]))
	Nil
})

test("split_negative_test", () {
	assert(list.split([0, 1, 2, 3, 4], -2) == ([], [0, 1, 2, 3, 4]))
	Nil
})

test("split_at_one_test", () {
	assert(list.split([0, 1, 2, 3, 4], 1) == ([0], [1, 2, 3, 4]))
	Nil
})

test("split_at_three_test", () {
	assert(list.split([0, 1, 2, 3, 4], 3) == ([0, 1, 2], [3, 4]))
	Nil
})

test("split_beyond_length_test", () {
	assert(list.split([0, 1, 2, 3, 4], 9) == ([0, 1, 2, 3, 4], []))
	Nil
})

test("split_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles + 10, -1, [], list.prepend)
		|> list.split(recursion_test_cycles + 1)
	Nil
})

test("split_while_empty_test", () {
	assert(list.split_while([], (x) { x <= 5 }) == ([], []))
	Nil
})

test("split_while_all_match_test", () {
	assert(
		list.split_while([1, 2, 3, 4, 5], (x) { x <= 5 }) ==
			([1, 2, 3, 4, 5], [])
	)
	Nil
})

test("split_while_none_match_test", () {
	assert(
		list.split_while([1, 2, 3, 4, 5], (x) { x == 2 }) ==
			([], [1, 2, 3, 4, 5])
	)
	Nil
})

test("split_while_partial_match_test", () {
	assert(
		list.split_while([1, 2, 3, 4, 5], (x) { x <= 3 }) == ([1, 2, 3], [4, 5])
	)
	Nil
})

test("split_while_negative_condition_test", () {
	assert(
		list.split_while([1, 2, 3, 4, 5], (x) { x <= -3 }) ==
			([], [1, 2, 3, 4, 5])
	)
	Nil
})

test("split_while_tco_test", () {
	(a, b) = list.split_while(list.range(1, big), (x) { x <= 10 })
	assert(list.length(a) == 10)
	assert(list.length(b) == big - 10)
})

test("key_find_first_test", () {
	proplist = [(0, "1"), (1, "2")]

	assert(list.key_find(proplist, 0) == Some("1"))
	Nil
})

test("key_find_second_test", () {
	proplist = [(0, "1"), (1, "2")]

	assert(list.key_find(proplist, 1) == Some("2"))
	Nil
})

test("key_find_not_found_test", () {
	proplist = [(0, "1"), (1, "2")]

	assert(list.key_find(proplist, 2) == None)
	Nil
})

test("key_filter_zero_test", () {
	proplist = [(0, "1"), (1, "2"), (0, "3"), (1, "4"), (2, "5")]

	assert(list.key_filter(proplist, 0) == ["1", "3"])
	Nil
})

test("key_filter_one_test", () {
	proplist = [(0, "1"), (1, "2"), (0, "3"), (1, "4"), (2, "5")]

	assert(list.key_filter(proplist, 1) == ["2", "4"])
	Nil
})

test("key_filter_two_test", () {
	proplist = [(0, "1"), (1, "2"), (0, "3"), (1, "4"), (2, "5")]

	assert(list.key_filter(proplist, 2) == ["5"])
	Nil
})

test("key_filter_not_found_test", () {
	proplist = [(0, "1"), (1, "2"), (0, "3"), (1, "4"), (2, "5")]

	assert(list.key_filter(proplist, 3) == [])
	Nil
})

test("key_pop_a_test", () {
	assert(list.key_pop([("a", 0), ("b", 1)], "a") == Some((0, [("b", 1)])))
	Nil
})

test("key_pop_b_test", () {
	assert(list.key_pop([("a", 0), ("b", 1)], "b") == Some((1, [("a", 0)])))
	Nil
})

test("key_pop_not_found_test", () {
	assert(list.key_pop([("a", 0), ("b", 1)], "c") == None)
	Nil
})

test("key_set_existing_test", () {
	assert(list.key_set([(5, 0), (4, 1)], 4, 100) == [(5, 0), (4, 100)])
	Nil
})

test("key_set_new_test", () {
	assert(list.key_set([(5, 0), (4, 1)], 1, 100) == [(5, 0), (4, 1), (1, 100)])
	Nil
})

test("key_set_tco_test", () {
	// TCO test
	recursion_test_cycles_list = int.range(
		recursion_test_cycles
		-1
		[]
		list.prepend
	)
	list.zip(recursion_test_cycles_list, recursion_test_cycles_list)
		|> list.key_set(0, 0)
	Nil
})

test("each_test", () {
	assert(list.each([1, 1, 1], (x) {
		1 = x
		Nil
	}) == Nil)
	Nil
})

test("each_tco_test", () {
	// TCO test
	list.each(list.repeat(1, recursion_test_cycles), (x) {
		1 = x
		Nil
	})
	Nil
})

test("try_each_ok_test", () {
	Ok(Nil) = list.try_each([1, 1, 1], (x) {
		assert(x == 1)
		Ok(Nil)
	})
	Nil
})

test("try_each_stops_on_error_test", () {
	// `try_each` actually stops when `fun` returns error
	Error(1) = list.try_each([1, 2, 3], (x) {
		assert(x == 1)
		Error(x)
	})
	Nil
})

test("try_each_tco_test", () {
	// TCO test
	Ok(Nil) = list.repeat(1, recursion_test_cycles) |> list.try_each((_) {
		Ok(Nil)
	})
	Nil
})

test("partition_test", () {
	assert(
		list.partition([1, 2, 3, 4, 5, 6, 7], int.is_odd) ==
			([1, 3, 5, 7], [2, 4, 6])
	)
	Nil
})

test("partition_tco_test", () {
	(evens, odds) = list.partition(list.range(1, big), int.is_even)
	assert(list.length(evens) == big / 2)
	assert(list.length(odds) == big / 2)
})

test("permutations_two_test", () {
	assert(list.permutations([1, 2]) == [[1, 2], [2, 1]])
	Nil
})

test("permutations_three_test", () {
	assert(
		list.permutations([1, 2, 3]) ==
			[[1, 2, 3], [1, 3, 2], [2, 1, 3], [2, 3, 1], [3, 1, 2], [3, 2, 1]]
	)
	Nil
})

test("permutations_four_test", () {
	assert(
		list.permutations([1, 2, 3, 4]) ==
			[
				[1, 2, 3, 4]
				[1, 2, 4, 3]
				[1, 3, 2, 4]
				[1, 3, 4, 2]
				[1, 4, 2, 3]
				[1, 4, 3, 2]
				[2, 1, 3, 4]
				[2, 1, 4, 3]
				[2, 3, 1, 4]
				[2, 3, 4, 1]
				[2, 4, 1, 3]
				[2, 4, 3, 1]
				[3, 1, 2, 4]
				[3, 1, 4, 2]
				[3, 2, 1, 4]
				[3, 2, 4, 1]
				[3, 4, 1, 2]
				[3, 4, 2, 1]
				[4, 1, 2, 3]
				[4, 1, 3, 2]
				[4, 2, 1, 3]
				[4, 2, 3, 1]
				[4, 3, 1, 2]
				[4, 3, 2, 1]
			]
	)
	Nil
})

test("permutations_strings_test", () {
	assert(list.permutations(["a", "b"]) == [["a", "b"], ["b", "a"]])
	Nil
})

test("permutations_duplicates_two_test", () {
	assert(list.permutations([1, 1]) == [[1, 1], [1, 1]])
	Nil
})

test("permutations_duplicates_three_test", () {
	assert(
		list.permutations([1, 1, 1]) ==
			[[1, 1, 1], [1, 1, 1], [1, 1, 1], [1, 1, 1], [1, 1, 1], [1, 1, 1]]
	)
	Nil
})

test("permutations_partial_duplicates_test", () {
	assert(
		list.permutations([1, 2, 2]) ==
			[[1, 2, 2], [1, 2, 2], [2, 1, 2], [2, 2, 1], [2, 1, 2], [2, 2, 1]]
	)
	Nil
})

test("permutations_all_same_four_test", () {
	assert(
		list.permutations(["a", "a", "a", "a"]) ==
			[
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
				["a", "a", "a", "a"]
			]
	)
	Nil
})

test("window_two_test", () {
	assert(list.window([1, 2, 3], 2) == [[1, 2], [2, 3]])
	Nil
})

test("window_three_test", () {
	assert(list.window([1, 2, 3], 3) == [[1, 2, 3]])
	Nil
})

test("window_four_test", () {
	assert(list.window([1, 2, 3], 4) == [])
	Nil
})

test("window_five_elements_test", () {
	assert(list.window([1, 2, 3, 4, 5], 3) == [[1, 2, 3], [2, 3, 4], [3, 4, 5]])
	Nil
})

test("window_zero_test", () {
	assert(list.window([1, 2, 3], 0) == [])
	Nil
})

test("window_negative_test", () {
	assert(list.window([1, 2, 3], -1) == [])
	Nil
})

test("window_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles, -1, [], list.prepend) |> list.window(2)
	Nil
})

test("window_by_2_test", () {
	assert(list.window_by_2([1, 2, 3, 4]) == [(1, 2), (2, 3), (3, 4)])
	Nil
})

test("window_by_2_single_test", () {
	assert(list.window_by_2([1]) == [])
	Nil
})

test("drop_while_test", () {
	assert(list.drop_while([1, 2, 3, 4], (x) { x < 3 }) == [3, 4])
	Nil
})

test("drop_while_tco_test", () {
	// TCO test
	int.range(recursion_test_cycles + 10, -1, [], list.prepend)
		|> list.drop_while((x) { x < recursion_test_cycles + 1 })
	Nil
})

test("take_while_test", () {
	assert(list.take_while([1, 2, 3, 2, 4], (x) { x < 3 }) == [1, 2])
	Nil
})

test("take_while_tco_test", () {
	assert(
		list.length(list.take_while(list.range(1, big), (x) { x < big })) ==
			big - 1
	)
})

test("chunk_test", () {
	assert(list.chunk([1, 2, 3], (n) { n % 2 }) == [[1], [2], [3]])
	Nil
})

test("chunk_adjacent_test", () {
	assert(
		list.chunk([1, 2, 2, 3, 4, 4, 6, 7, 7], (n) { n % 2 }) ==
			[[1], [2, 2], [3], [4, 4, 6], [7, 7]]
	)
	Nil
})

test("chunk_tco_test", () {
	assert(
		list.length(list.chunk(list.range(1, big), (x) { x / 10 })) ==
			big / 10 + 1
	)
})

test("sized_chunk_test", () {
	assert(list.sized_chunk([1, 2, 3, 4, 5, 6], 2) == [[1, 2], [3, 4], [5, 6]])
	Nil
})

test("sized_chunk_remainder_test", () {
	assert(
		list.sized_chunk([1, 2, 3, 4, 5, 6, 7, 8], 3) ==
			[[1, 2, 3], [4, 5, 6], [7, 8]]
	)
	Nil
})

test("sized_chunk_tco_test", () {
	assert(list.length(list.sized_chunk(list.range(1, big), 3)) == 333_334)
})

test("reduce_empty_test", () {
	assert(list.reduce([], (x, y) { x + y }) == None)
	Nil
})

test("reduce_sum_test", () {
	assert(list.reduce([1, 2, 3, 4, 5], (x, y) { x + y }) == Some(15))
	Nil
})

test("scan_empty_test", () {
	assert(list.scan([], 0, (acc, i) { i + acc }) == [])
	Nil
})

test("scan_basic_test", () {
	assert(
		list.scan([1, 2, 3, 4], 0, (acc, i) { 2 * i + acc }) == [2, 6, 12, 20]
	)
	Nil
})

test("scan_list_test", () {
	assert(list.scan([1, 2, 3, 4], [], (acc, i) {
		int.is_even(i) |> {
			True  -> ["Even", ..acc]
			False -> ["Odd", ..acc]
		}
	}) == [["Odd"], ["Even", "Odd"], ["Odd", "Even", "Odd"], ["Even", "Odd", "Even", "Odd"]])
	Nil
})

test("scan_tco_test", () {
	assert(list.last(list.scan(list.repeat(1, big), 0, int.add)) == Some(big))
})

test("last_empty_test", () {
	assert(list.last([]) == None)
	Nil
})

test("last_ok_test", () {
	assert(list.last([1, 2, 3, 4, 5]) == Some(5))
	Nil
})

test("combinations_zero_test", () {
	assert(list.combinations([1, 2, 3], 0) == [[]])
	Nil
})

test("combinations_one_test", () {
	assert(list.combinations([1, 2, 3], 1) == [[1], [2], [3]])
	Nil
})

test("combinations_two_test", () {
	assert(list.combinations([1, 2, 3], 2) == [[1, 2], [1, 3], [2, 3]])
	Nil
})

test("combinations_three_test", () {
	assert(list.combinations([1, 2, 3], 3) == [[1, 2, 3]])
	Nil
})

test("combinations_four_test", () {
	assert(list.combinations([1, 2, 3], 4) == [])
	Nil
})

test("combinations_four_elements_test", () {
	assert(
		list.combinations([1, 2, 3, 4], 3) ==
			[[1, 2, 3], [1, 2, 4], [1, 3, 4], [2, 3, 4]]
	)
	Nil
})

test("combination_pairs_single_test", () {
	assert(list.combination_pairs([1]) == [])
	Nil
})

test("combination_pairs_two_test", () {
	assert(list.combination_pairs([1, 2]) == [(1, 2)])
	Nil
})

test("combination_pairs_three_test", () {
	assert(list.combination_pairs([1, 2, 3]) == [(1, 2), (1, 3), (2, 3)])
	Nil
})

test("combination_pairs_four_test", () {
	assert(
		list.combination_pairs([1, 2, 3, 4]) ==
			[(1, 2), (1, 3), (1, 4), (2, 3), (2, 4), (3, 4)]
	)
	Nil
})

test("combination_pairs_tco_test", () {
	// TCO test
	int.range(200, -1, [], list.prepend) |> list.combination_pairs()
	Nil
})

test("interleave_two_test", () {
	assert(list.interleave([[1, 2], [101, 102]]) == [1, 101, 2, 102])
	Nil
})

test("interleave_three_test", () {
	assert(
		list.interleave([[1, 2], [101, 102], [201, 202]]) ==
			[1, 101, 201, 2, 102, 202]
	)
	Nil
})

test("interleave_leftover_first_test", () {
	// Left over elements are added at the end
	assert(list.interleave([[1, 2, 3], [101, 102]]) == [1, 101, 2, 102, 3])
	Nil
})

test("interleave_leftover_second_test", () {
	assert(list.interleave([[1, 2], [101, 102, 103]]) == [1, 101, 2, 102, 103])
	Nil
})

test("transpose_two_test", () {
	assert(
		list.transpose([[1, 2, 3], [101, 102, 103]]) ==
			[[1, 101], [2, 102], [3, 103]]
	)
	Nil
})

test("transpose_three_test", () {
	assert(
		list.transpose([[1, 2, 3], [101, 102, 103], [201, 202, 203]]) ==
			[[1, 101, 201], [2, 102, 202], [3, 103, 203]]
	)
	Nil
})

test("transpose_leftover_second_test", () {
	// Left over elements are still returned
	assert(
		list.transpose([[1, 2], [101, 102, 103]]) == [[1, 101], [2, 102], [103]]
	)
	Nil
})

test("transpose_leftover_mixed_test", () {
	assert(
		list.transpose([[1, 2, 3], [101, 102], [201, 202, 203]]) ==
			[[1, 101, 201], [2, 102, 202], [3, 203]]
	)
	Nil
})

test("max_empty_test", () {
	assert(list.max([], int.compare) == None)
	Nil
})

test("max_int_test", () {
	assert(list.max([1, 3, 2], int.compare) == Some(3))
	Nil
})

test("max_float_test", () {
	assert(list.max([-.1.0, 1.2, 1.104], float.compare) == Some(1.2))
	Nil
})

test("max_string_test", () {
	assert(list.max(["a", "c", "b"], string.compare) == Some("c"))
	Nil
})

// Tests for functions that are new in std, or that changed from the old
// library.

big = 1_000_000

test("at_test", () {
	assert(list.at([10, 20, 30], 0) == Some(10))
	assert(list.at([10, 20, 30], 2) == Some(30))
	assert(list.at([10, 20, 30], 3) == None)
	assert(list.at([10, 20, 30], -1) == None)
	assert(list.at([], 0) == None)
})

test("at_tco_test", () {
	assert(list.at(list.range(0, big - 1), big - 1) == Some(big - 1))
})

test("index_of_test", () {
	assert(list.index_of(["a", "b", "c", "b"], "b") == Some(1))
	assert(list.index_of(["a", "b"], "z") == None)
	assert(list.index_of([], 1) == None)
})

test("index_of_tco_test", () {
	assert(list.index_of(list.range(1, big), big) == Some(big - 1))
})

test("last_tco_test", () { assert(list.last(list.range(1, big)) == Some(big)) })

test("concat_test", () {
	assert(list.concat([[1], [], [2, 3], [4]]) == [1, 2, 3, 4])
	assert(list.concat([]) == [])
	assert(list.concat([[], []]) == [])
})

test("concat_tco_test", () {
	assert(list.length(list.concat(list.repeat([1, 2], 500_000))) == big)
})

test("fold_right_tco_test", () {
	assert(
		list.fold_right(list.range(1, big), 0, (acc, x) { acc + x }) ==
			500_000_500_000
	)
	assert(list.fold_right([1, 2, 3], [], (acc, x) { [x, ..acc] }) == [1, 2, 3])
})

test("flat_map_tco_test", () {
	assert(
		list.length(list.flat_map(list.range(1, 500_000), (x) { [x, x] })) ==
			big
	)
})

test("zip_unzip_tco_test", () {
	xs = list.range(1, big)
	(a, b) = list.unzip(list.zip(xs, xs))
	assert(a == xs)
	assert(b == xs)
})

test("range_test", () {
	assert(list.range(1, 4) == [1, 2, 3, 4])
	assert(list.range(4, 1) == [4, 3, 2, 1])
	assert(list.range(0, 0) == [0])
	assert(list.range(-2, 1) == [-2, -1, 0, 1])
	assert(list.length(list.range(1, big)) == big)
})

test("sort_stable_test", () {
	pairs = [(2, "a"), (1, "b"), (2, "c"), (1, "d"), (0, "e"), (2, "f")]
	by_first = (x: (Int, String), y: (Int, String)) { int.compare(x.0, y.0) }
	assert(
		list.sort(pairs, by_first) ==
			[(0, "e"), (1, "b"), (1, "d"), (2, "a"), (2, "c"), (2, "f")]
	)
	// Runs that fall with equal elements in them must stay stable.
	falling = [(3, "a"), (2, "b"), (2, "c"), (1, "d")]
	assert(
		list.sort(falling, by_first) == [(1, "d"), (2, "b"), (2, "c"), (3, "a")]
	)
})

test("sort_reverse_test", () {
	assert(
		list.sort([1, 5, 2, 4, 3], order.reverse(int.compare)) ==
			[5, 4, 3, 2, 1]
	)
})

test("sort_big_test", () {
	// A shuffled range: 7 has no factor in common with `big`.
	xs = list.map(list.range(0, big - 1), (i) { i * 7 % big })
	assert(list.sort(xs, int.compare) == list.range(0, big - 1))
})

test("sort_stable_big_test", () {
	xs = list.map(list.range(0, 99_999), (i) { (i % 10, i) })
	sorted = list.sort(xs, (a: (Int, Int), b: (Int, Int)) {
		int.compare(a.0, b.0)
	})
	// Within each key, the second parts keep their order.
	ok = list.window_by_2(sorted) |> list.all((p) {
		(a, b) = p
		a.0 < b.0 || { a.0 == b.0 && a.1 < b.1 }
	})
	assert(ok)
})

test("sort_effects_test", () {
	// The compare function may raise errors.
	r = result.catch(() { list.sort([3, 1, 2], (a, b) { fail.raise("no") }) })
	assert(r == Error("no"))
})

test("map_effects_test", () {
	r = result.catch(() {
		list.map([1, 2, 3], (x) {
			x |> {
				2 -> fail.raise("two")
				_ -> x
			}
		})
	})
	assert(r == Error("two"))
	assert(result.catch(() { list.map([1, 2], (x) { x }) }) == Ok([1, 2]))
})

test("min_max_test", () {
	assert(list.max([3, 1, 4, 1, 5], int.compare) == Some(5))
	assert(list.min([3, 1, 4, 1, 5], int.compare) == Some(1))
	assert(list.max([], int.compare) == None)
	assert(list.min(["b", "a", "c"], string.compare) == Some("a"))
	// The first of equal elements is given.
	by_first = (a: (Int, String), b: (Int, String)) { int.compare(a.0, b.0) }
	assert(list.max([(1, "a"), (2, "b"), (2, "c")], by_first) == Some((2, "b")))
	assert(list.min([(1, "a"), (2, "b"), (1, "c")], by_first) == Some((1, "a")))
})

test("contains_any_label_test", () {
	assert(list.contains([1, 2, 3], 2))
	assert(core.not(list.contains([1, 2, 3], 4)))
})

test("window_test", () {
	assert(list.window([1, 2, 3, 4, 5], 3) == [[1, 2, 3], [2, 3, 4], [3, 4, 5]])
	assert(list.window([1, 2], 3) == [])
	assert(list.window([1, 2], 0) == [])
})

test("index_map_tco_test", () {
	assert(
		list.last(list.index_map(list.repeat(0, big), (_, i) { i })) ==
			Some(big - 1)
	)
})

test("count_tco_test", () {
	assert(list.count(list.range(1, big), int.is_even) == big / 2)
})

test("any_all_tco_test", () {
	xs = list.range(1, big)
	assert(list.all(xs, (x) { x > 0 }))
	assert(list.any(xs, (x) { x == big }))
})

test("key_find_tco_test", () {
	pairs = list.map(list.range(1, big), (i) { (i, i * 2) })
	assert(list.key_find(pairs, big) == Some(2 * big))
	assert(list.key_find(pairs, 0) == None)
})

test("key_filter_test", () {
	assert(list.key_filter([(1, "a"), (2, "b"), (1, "c")], 1) == ["a", "c"])
})

test("transpose_ragged_test", () {
	assert(list.transpose([[1, 2, 3], [4], [5, 6]]) == [[1, 4, 5], [2, 6], [3]])
	assert(list.transpose([]) == [])
})

test("interleave_test", () {
	assert(
		list.interleave([[1, 2], [101, 102], [201]]) == [1, 101, 201, 2, 102]
	)
})

test("group_test_order", () {
	g = list.group([1, 2, 3, 4, 5, 6], (x) { x % 3 })
	assert(dict.get(g, 0) == Some([3, 6]))
	assert(dict.get(g, 1) == Some([1, 4]))
	assert(dict.get(g, 2) == Some([2, 5]))
	assert(dict.size(g) == 3)
})

test("group_tco_test", () {
	g = list.group(list.range(1, big), (x) { x % 2 })
	assert(dict.size(g) == 2)
	assert(option.map(dict.get(g, 0), list.length) == Some(big / 2))
})

test("labels_test", () {
	assert(list.fold([1, 2, 3], 0, int.add) == 6)
	assert(list.map([1], (x) { x + 1 }) == [2])
	assert(list.filter([1, 2, 3, 4], int.is_odd) == [1, 3])
	assert(list.find([1, 2, 3], (x) { x > 1 }) == Some(2))
	assert(list.take([1, 2, 3], 2) == [1, 2])
})

User {
	User(name: String, age: Int)
}

test("known_types_test", () {
	// The field of `u` is known from the type of `list.map`, with no annotation.
	users = [User(name: "ann", age: 30), User(name: "bob", age: 25)]
	assert(list.map(users, (u) { u.name }) == ["ann", "bob"])
	assert(
		list.filter(users, (u) { u.age > 26 }) == [User(name: "ann", age: 30)]
	)
	assert(
		list.sort(users, (a, b) { int.compare(a.age, b.age) }) ==
			[User(name: "bob", age: 25), User(name: "ann", age: 30)]
	)
	assert(list.fold(users, 0, (acc, u) { acc + u.age }) == 55)
})

Counter {
	Counter(step: Int)
}

Yield(A) {
	Yield(emit: (A) -> Nil)
}

// A generator made with `i/control` (§9.4).
collect(A, N) = (body: () -> Nil ! Yield(A) | N) -> List(A) ! N {
	control.boundary((b) {
		! Yield(A) = Yield(emit: (x) {
			control.suspend(b, (k) { [x, ..control.resume(k, Nil)] })
		})
		body()
		[]
	})
}

test("environment_callbacks_test", () {
	// A callback may need a service; `list.map` passes the environment on.
	! Counter = Counter(step: 7)
	assert(list.map([1, 2, 3], (x, c ! Counter) { x + c.step }) == [8, 9, 10])
})

test("generator_callbacks_test", () {
	ys = collect((y ! Yield(Int)) {
		list.each([1, 2, 3], (x) { y.emit(x * 10) })
	})
	assert(ys == [10, 20, 30])
	zs = collect((y ! Yield(Int)) {
		list.fold([1, 2, 3], 0, (acc, x) {
			y.emit(acc)
			acc + x
		})
		Nil
	})
	assert(zs == [0, 1, 3])
})
