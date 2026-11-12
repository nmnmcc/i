(
	testlib = (test) = std/test
	i/core
	i/fail
	dict = std/dict
	int = std/int
	list = std/list
	string = std/string
	option = (Some, None) = std/option
)

test("from_list_size_test", () {
	assert([(4, 0), (1, 0)] |> dict.from_list |> dict.size == 2)
	Nil
})

test("from_list_duplicate_key_test", () {
	assert(dict.from_list([(1, 0), (1, 1)]) == dict.from_list([(1, 1)]))
	Nil
})

test("from_list_inequality_value_test", () {
	assert(dict.from_list([(1, 0), (2, 1)]) != dict.from_list([(1, 0), (2, 2)]))
	Nil
})

test("from_list_inequality_key_test", () {
	assert(dict.from_list([(1, 0), (2, 1)]) != dict.from_list([(1, 0), (3, 1)]))
	Nil
})

test("has_key_empty_test", () {
	assert(core.not([] |> dict.from_list |> dict.has(1)))
	Nil
})

test("has_key_present_test", () {
	assert([(1, 0)] |> dict.from_list |> dict.has(1))
	Nil
})

test("has_key_present_multiple_test", () {
	assert([(4, 0), (1, 0)] |> dict.from_list |> dict.has(1))
	Nil
})

test("has_key_absent_test", () {
	assert(core.not([(4, 0), (1, 0)] |> dict.from_list |> dict.has(0)))
	Nil
})

test("new_size_test", () {
	assert(dict.size(dict.new()) == 0)
	Nil
})

test("new_to_list_test", () {
	assert(dict.to_list(dict.new()) == [])
	Nil
})

Key = A | B | C

A {
	A
}

B {
	B
}

C {
	C
}

test("get_int_key_first_test", () {
	proplist = [(4, 0), (1, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, 4) == Some(0))
	Nil
})

test("get_int_key_second_test", () {
	proplist = [(4, 0), (1, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, 1) == Some(1))
	Nil
})

test("get_int_key_missing_test", () {
	proplist = [(4, 0), (1, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, 2) == None)
	Nil
})

test("get_custom_key_a_test", () {
	proplist = [(A, 0), (B, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, A) == Some(0))
	Nil
})

test("get_custom_key_b_test", () {
	proplist = [(A, 0), (B, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, B) == Some(1))
	Nil
})

test("get_custom_key_missing_test", () {
	proplist = [(A, 0), (B, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, C) == None)
	Nil
})

test("get_bit_array_key_first_test", () {
	proplist = [(<<1, 2, 3>>, 0), (<<3, 2, 1>>, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, <<1, 2, 3>>) == Some(0))
	Nil
})

test("get_bit_array_key_second_test", () {
	proplist = [(<<1, 2, 3>>, 0), (<<3, 2, 1>>, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, <<3, 2, 1>>) == Some(1))
	Nil
})

test("get_bit_array_key_missing_test", () {
	proplist = [(<<1, 2, 3>>, 0), (<<3, 2, 1>>, 1)]
	m = dict.from_list(proplist)

	assert(dict.get(m, <<1, 3, 2>>) == None)
	Nil
})

test("insert_test", () {
	assert(
		dict.new()
			|> dict.insert("a", 0)
			|> dict.insert("b", 1)
			|> dict.insert("c", 2) == dict.from_list([
				("a", 0)
				("b", 1)
				("c", 2)
			])
	)
	Nil
})

test("map_values_test", () {
	assert(
		[(1, 0), (2, 1), (3, 2)] |> dict.from_list |> dict.map_values((k, v) {
			k + v
		}) == dict.from_list([(1, 1), (2, 3), (3, 5)])
	)
	Nil
})

test("keys_test", () {
	assert(
		[("a", 0), ("b", 1), ("c", 2)]
			|> dict.from_list
			|> dict.keys
			|> list.sort(string.compare) == ["a", "b", "c"]
	)
	Nil
})

test("values_test", () {
	assert(
		[("a", 0), ("b", 1), ("c", 2)]
			|> dict.from_list
			|> dict.values
			|> list.sort(int.compare) == [0, 1, 2]
	)
	Nil
})

test("take_test", () {
	assert(
		[("a", 0), ("b", 1), ("c", 2)]
			|> dict.from_list
			|> dict.take(["a", "b", "d"]) == dict.from_list([
				("a", 0)
				("b", 1)
			])
	)
	Nil
})

test("drop_test", () {
	assert(
		[("a", 0), ("b", 1), ("c", 2)]
			|> dict.from_list
			|> dict.drop(["a", "b", "d"]) == dict.from_list([("c", 2)])
	)
	Nil
})

test("merge_same_key_first_wins_test", () {
	a = dict.from_list([("a", 2)])
	b = dict.from_list([("a", 0)])

	assert(dict.merge(a, b) == dict.from_list([("a", 0)]))
	Nil
})

test("merge_same_key_second_wins_test", () {
	a = dict.from_list([("a", 2)])
	b = dict.from_list([("a", 0)])

	assert(dict.merge(b, a) == dict.from_list([("a", 2)]))
	Nil
})

test("merge_first_test", () {
	a = dict.from_list([("a", 2), ("c", 4), ("d", 3)])
	b = dict.from_list([("a", 0), ("b", 1), ("c", 2)])

	assert(
		dict.merge(a, b) ==
			dict.from_list([("a", 0), ("b", 1), ("c", 2), ("d", 3)])
	)
	Nil
})

test("merge_second_test", () {
	a = dict.from_list([("a", 2), ("c", 4), ("d", 3)])
	b = dict.from_list([("a", 0), ("b", 1), ("c", 2)])

	assert(
		dict.merge(b, a) ==
			dict.from_list([("a", 2), ("b", 1), ("c", 4), ("d", 3)])
	)
	Nil
})

test("delete_test", () {
	assert(
		[("a", 0), ("b", 1), ("c", 2)]
			|> dict.from_list
			|> dict.delete("a")
			|> dict.delete("d") == dict.from_list([("b", 1), ("c", 2)])
	)
	Nil
})

test("upsert_existing_test", () {
	d = dict.from_list([("a", 0), ("b", 1), ("c", 2)])

	inc_or_zero = (x) {
		x |> {
			Some(i) -> i + 1
			None    -> 0
		}
	}

	assert(
		dict.upsert(d, "a", inc_or_zero) ==
			dict.from_list([("a", 1), ("b", 1), ("c", 2)])
	)
	Nil
})

test("upsert_existing_b_test", () {
	d = dict.from_list([("a", 0), ("b", 1), ("c", 2)])

	inc_or_zero = (x) {
		x |> {
			Some(i) -> i + 1
			None    -> 0
		}
	}

	assert(
		dict.upsert(d, "b", inc_or_zero) ==
			dict.from_list([("a", 0), ("b", 2), ("c", 2)])
	)
	Nil
})

test("upsert_new_test", () {
	d = dict.from_list([("a", 0), ("b", 1), ("c", 2)])

	inc_or_zero = (x) {
		x |> {
			Some(i) -> i + 1
			None    -> 0
		}
	}

	assert(
		dict.upsert(d, "z", inc_or_zero) ==
			dict.from_list([("a", 0), ("b", 1), ("c", 2), ("z", 0)])
	)
	Nil
})

test("fold_sum_test", () {
	d = dict.from_list([("a", 0), ("b", 1), ("c", 2), ("d", 3)])

	add = (acc, _, v) { v + acc }

	assert(dict.fold(d, 0, add) == 6)
	Nil
})

test("fold_keys_test", () {
	d = dict.from_list([("a", 0), ("b", 1), ("c", 2), ("d", 3)])

	prepend = (acc, k, _) { list.prepend(acc, k) }

	assert(
		d |> dict.fold([], prepend) |> list.sort(string.compare) == [
				"a"
				"b"
				"c"
				"d"
			]
	)
	Nil
})

test("fold_empty_test", () {
	add = (acc, _, v) { v + acc }

	assert(dict.fold(dict.from_list([]), 0, add) == 0)
	Nil
})

test("each_test", () {
	d = dict.from_list([("a", 1), ("b", 2), ("c", 3), ("d", 4)])

	assert(dict.each(d, (k, v) {
		True = (k, v) |> {
			("a", 1) | ("b", 2) | ("c", 3) | ("d", 4) -> True
			_                                         -> False
		}
		Nil
	}) == Nil)
	Nil
})

range = (start, end, a) {
	end - start |> {
		n ? n < 1 -> a
		_         -> range(start, end - 1, [end - 1, ..a])
	}
}

list_to_map = (xs) { xs |> list.map((n) { (n, n) }) |> dict.from_list }

grow_and_shrink_map = (initial_size, final_size) {
	range(0, initial_size, []) |> list_to_map |> (m) {
		list.fold(range(final_size, initial_size, []), m, (map, item) {
			dict.delete(map, item)
		})
	}
}

// maps should be equal even if the insert/removal order was different

test("insert_order_equality_small_test", () {
	assert(grow_and_shrink_map(8, 2) == grow_and_shrink_map(4, 2))
	Nil
})

test("insert_order_equality_medium_test", () {
	assert(grow_and_shrink_map(17, 10) == grow_and_shrink_map(12, 10))
	Nil
})

test("insert_order_equality_large_test", () {
	assert(grow_and_shrink_map(2000, 1000) == grow_and_shrink_map(1000, 1000))
	Nil
})

// ensure operations on a map don't mutate it

test("persistence_test", () {
	a = list_to_map([0])
	_ = dict.insert(a, 0, 5)
	_ = dict.insert(a, 1, 6)
	_ = dict.delete(a, 0)
	assert(dict.get(a, 0) == Some(0))
	Nil
})

// using maps as keys should work (tests hash function)

test("map_as_key_test", () {
	l = range(0, 1000, [])
	a = list_to_map(l)
	a2 = list_to_map(list.reverse(l))
	a3 = grow_and_shrink_map(2000, 1000)
	b = grow_and_shrink_map(60, 50)
	c = grow_and_shrink_map(50, 20)
	d = grow_and_shrink_map(2, 2)

	map1 = dict.new()
		|> dict.insert(a, "a")
		|> dict.insert(b, "b")
		|> dict.insert(c, "c")
		|> dict.insert(d, "d")

	assert(dict.get(map1, a) == Some("a"))
	assert(dict.get(map1, a2) == Some("a"))
	assert(dict.get(map1, a3) == Some("a"))
	assert(dict.get(map1, b) == Some("b"))
	assert(dict.get(map1, c) == Some("c"))
	assert(dict.get(map1, d) == Some("d"))
	assert(dict.get(dict.insert(map1, a2, "a2"), a) == Some("a2"))
	assert(dict.get(dict.insert(map1, a3, "a3"), a) == Some("a3"))
	Nil
})

test("large_n_get_test", () {
	n = 10_000
	l = range(0, n, [])

	m = list_to_map(l)
	list.map(l, (i) { assert(dict.get(m, i) == Some(i)) })
	Nil
})

test("large_n_after_shrink_test", () {
	n = 10_000
	l = range(0, n, [])

	m = grow_and_shrink_map(n, 0)
	list.map(l, (i) { assert(dict.get(m, i) == None) })
	Nil
})

test("size_basic_test", () {
	n = 1000
	m = list_to_map(range(0, n, []))
	assert(dict.size(m) == n)
	Nil
})

test("size_after_shrink_test", () {
	n = 1000
	m = grow_and_shrink_map(n, n / 2)
	assert(dict.size(m) == n / 2)
	Nil
})

test("size_empty_after_delete_test", () {
	n = 1000
	m = grow_and_shrink_map(n, 0) |> dict.delete(0)
	assert(dict.size(m) == 0)
	Nil
})

test("size_insert_existing_test", () {
	m = list_to_map(range(0, 18, []))

	assert(dict.size(dict.insert(m, 1, 99)) == 18)
	Nil
})

test("size_insert_existing_2_test", () {
	m = list_to_map(range(0, 18, []))

	assert(dict.size(dict.insert(m, 2, 99)) == 18)
	Nil
})

test("is_empty_new_test", () {
	assert(dict.is_empty(dict.new()))
	Nil
})

test("is_empty_with_element_test", () {
	assert(core.not(dict.new() |> dict.insert(1, 10) |> dict.is_empty()))
	Nil
})

test("is_empty_after_delete_test", () {
	assert(
		dict.new() |> dict.insert(1, 10) |> dict.delete(1) |> dict.is_empty()
	)
	Nil
})

// https://github.com/gleam-lang/stdlib/issues/435

test("peters_bug_test", () {
	assert(
		dict.new()
			|> dict.insert(22, Nil)
			|> dict.insert(21, Nil)
			|> dict.insert(23, Nil)
			|> dict.insert(18, Nil)
			|> dict.insert(17, Nil)
			|> dict.insert(19, Nil)
			|> dict.insert(14, Nil)
			|> dict.insert(13, Nil)
			|> dict.insert(15, Nil)
			|> dict.insert(10, Nil)
			|> dict.insert(9, Nil)
			|> dict.insert(11, Nil)
			|> dict.insert(6, Nil)
			|> dict.insert(5, Nil)
			|> dict.insert(7, Nil)
			|> dict.insert(2, Nil)
			|> dict.insert(1, Nil)
			|> dict.insert(3, Nil)
			|> dict.get(0) == None
	)
	Nil
})

test("zero_must_be_contained_get_test", () {
	map = dict.new() |> dict.insert(0, Nil)

	assert(dict.get(map, 0) == Some(Nil))
	Nil
})

test("zero_must_be_contained_has_key_test", () {
	map = dict.new() |> dict.insert(0, Nil)

	assert(dict.has(map, 0) == True)
	Nil
})

test("empty_map_equality_first_test", () {
	map1 = dict.new()
	map2 = dict.from_list([(1, 2)])

	assert(map1 != map2)
	Nil
})

test("empty_map_equality_second_test", () {
	map1 = dict.new()
	map2 = dict.from_list([(1, 2)])

	assert(map2 != map1)
	Nil
})

test("extra_keys_equality_first_test", () {
	map1 = dict.from_list([(1, 2), (3, 4)])
	map2 = dict.from_list([(1, 2), (3, 4), (4, 5)])

	assert(map1 != map2)
	Nil
})

test("extra_keys_equality_second_test", () {
	map1 = dict.from_list([(1, 2), (3, 4)])
	map2 = dict.from_list([(1, 2), (3, 4), (4, 5)])

	assert(map2 != map1)
	Nil
})

test("combine_test", () {
	map1 = dict.from_list([("a", 3), ("b", 2)])
	map2 = dict.from_list([("a", 2), ("c", 3), ("d", 4)])

	assert(
		dict.combine(map1, map2, (one, other) { one - other }) ==
			dict.from_list([("a", 1), ("b", 2), ("c", 3), ("d", 4)])
	)
	Nil
})

test("combine_with_empty_first_test", () {
	map1 = dict.from_list([("a", 3), ("b", 2)])

	assert(dict.combine(map1, dict.new(), (one, _) { one }) == map1)
	Nil
})

test("combine_with_empty_second_test", () {
	map1 = dict.from_list([("a", 3), ("b", 2)])

	assert(dict.combine(dict.new(), map1, (one, _) { one }) == map1)
	Nil
})

test("combine_with_no_overlapping_keys_test", () {
	map1 = dict.from_list([("a", 1), ("b", 2)])
	map2 = dict.from_list([("c", 3), ("d", 4)])

	assert(
		dict.combine(map1, map2, (one, _) { one }) ==
			dict.from_list([("a", 1), ("b", 2), ("c", 3), ("d", 4)])
	)
	Nil
})

// Enums without fields all hash to 0 due to how the hash function works -
// we use this fact here to produce and test collisions.
//
// Object.keys() returns [] for variants without fields, so the hash always
// stays on it's initial value.
CollidingKey = CollidingKey1 | CollidingKey2

CollidingKey1 {
	CollidingKey1
}

CollidingKey2 {
	CollidingKey2
}

test("hash_collision_size_test", () {
	d = dict.new()
		|> dict.insert(CollidingKey1, 1)
		|> dict.insert(CollidingKey2, 2)

	assert(dict.size(d) == 2)
	Nil
})

test("hash_collision_get_first_test", () {
	d = dict.new()
		|> dict.insert(CollidingKey1, 1)
		|> dict.insert(CollidingKey2, 2)

	assert(dict.get(d, CollidingKey1) == Some(1))
	Nil
})

test("hash_collision_get_second_test", () {
	d = dict.new()
		|> dict.insert(CollidingKey1, 1)
		|> dict.insert(CollidingKey2, 2)

	assert(dict.get(d, CollidingKey2) == Some(2))
	Nil
})

test("hash_collision_equality_insert_order_test", () {
	d1 = dict.from_list([(CollidingKey1, 1), (CollidingKey2, 2)])
	d2 = dict.from_list([(CollidingKey2, 2), (CollidingKey1, 1)])

	assert(d1 == d2)
	Nil
})

test("hash_collision_map_as_key_test", () {
	d1 = dict.from_list([(CollidingKey1, 1), (CollidingKey2, 2)])
	d2 = dict.from_list([(CollidingKey2, 2), (CollidingKey1, 1)])
	d3 = dict.from_list([(CollidingKey1, -20), (CollidingKey2, 15)])

	outer = dict.new() |> dict.insert(d1, "inner") |> dict.insert(d3, "other")

	assert(dict.get(outer, d2) == Some("inner"))
	Nil
})

test("hash_collision_after_delete_size_test", () {
	d = dict.new()
		|> dict.insert(CollidingKey1, 1)
		|> dict.insert(CollidingKey2, 2)
	d = dict.delete(d, CollidingKey1)

	assert(dict.size(d) == 1)
	Nil
})

test("hash_collision_after_delete_get_first_test", () {
	d = dict.new()
		|> dict.insert(CollidingKey1, 1)
		|> dict.insert(CollidingKey2, 2)
	d = dict.delete(d, CollidingKey1)

	assert(dict.get(d, CollidingKey1) == None)
	Nil
})

test("hash_collision_after_delete_get_second_test", () {
	d = dict.new()
		|> dict.insert(CollidingKey1, 1)
		|> dict.insert(CollidingKey2, 2)
	d = dict.delete(d, CollidingKey1)

	assert(dict.get(d, CollidingKey2) == Some(2))
	Nil
})

Colour = Red | Green | Blue

Red {
	Red
}

Green {
	Green
}

Blue {
	Blue
}

test("hash_collision_fold_green_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	str = dict.fold(colours, "", (acc, _color, name) { acc <> name <> "\n" })
	assert(string.contains(str, "green\n"))
	Nil
})

test("hash_collision_fold_red_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	str = dict.fold(colours, "", (acc, _color, name) { acc <> name <> "\n" })
	assert(string.contains(str, "red\n"))
	Nil
})

test("hash_collision_fold_blue_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	str = dict.fold(colours, "", (acc, _color, name) { acc <> name <> "\n" })
	assert(string.contains(str, "blue\n"))
	Nil
})

test("hash_collision_map_size_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	uppercase = dict.map_values(colours, (_colour, name) {
		string.to_upper(name)
	})

	assert(dict.size(uppercase) == 3)
	Nil
})

test("hash_collision_map_red_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	uppercase = dict.map_values(colours, (_colour, name) {
		string.to_upper(name)
	})

	assert(dict.get(uppercase, Red) == Some("RED"))
	Nil
})

test("hash_collision_map_green_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	uppercase = dict.map_values(colours, (_colour, name) {
		string.to_upper(name)
	})

	assert(dict.get(uppercase, Green) == Some("GREEN"))
	Nil
})

test("hash_collision_map_blue_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	uppercase = dict.map_values(colours, (_colour, name) {
		string.to_upper(name)
	})

	assert(dict.get(uppercase, Blue) == Some("BLUE"))
	Nil
})

test("hash_collision_values_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	assert(
		list.sort(dict.values(colours), string.compare) ==
			["blue", "green", "red"]
	)
	Nil
})

test("hash_collision_keys_test", () {
	colours = dict.from_list([(Red, "red"), (Green, "green"), (Blue, "blue")])

	assert(
		dict.keys(colours)
			|> list.filter_map((k) { dict.get(colours, k) })
			|> list.sort(string.compare) == ["blue", "green", "red"]
	)
	Nil
})

// Random operations, checked against a list of pairs as a model. The
// numbers come from a fixed generator, so every run is the same.
test_random_operations = (
	initial_seed: Int
	num_ops: Int
	key_space: Int
	initial: dict.Dict(Int, Int)
) -> Nil {
	test_random_operations_loop(
		initial_seed
		prng(initial_seed)
		num_ops
		key_space
		dict.to_list(initial)
		initial
	)
}

test_random_operations_loop = (
	initial_seed: Int
	seed: Int
	remaining: Int
	key_space: Int
	proplist: List((Int, Int))
	d: dict.Dict(Int, Int)
) -> Nil {
	{ remaining > 0 } |> {
		False -> assert_dict_matches_proplist(d, proplist, initial_seed)
		True -> {
			seed1 = prng(seed)
			op_choice = seed1 % 2
			seed2 = prng(seed1)
			key = seed2 % key_space
			op_choice |> {
				// Insert
				0 -> test_random_operations_loop(
					initial_seed
					seed2
					remaining - 1
					key_space
					list.key_set(proplist, key, key * 2)
					dict.insert(d, key, key * 2)
				)
				// Delete
				_ -> {
					new_proplist = list.key_pop(proplist, key) |> {
						Some((_, rest)) -> rest
						None            -> proplist
					}
					test_random_operations_loop(
						initial_seed
						seed2
						remaining - 1
						key_space
						new_proplist
						dict.delete(d, key)
					)
				}
			}
		}
	}
}

run_many_random_tests = (
	count: Int
	ops_per_test: Int
	key_space: Int
	initial: dict.Dict(Int, Int)
) -> Nil {
	go = (n, seed) {
		n |> {
			0 -> Nil
			_ -> {
				test_random_operations(seed, ops_per_test, key_space, initial)
				go(n - 1, prng(seed + n))
			}
		}
	}
	go(count, 12345)
}

test("random_operations_small_test", () {
	run_many_random_tests(
		count:        100
		ops_per_test: 50
		key_space:    32
		initial:      dict.new()
	)
})

test("random_operations_medium_test", () {
	run_many_random_tests(
		count:        100
		ops_per_test: 50
		key_space:    200
		initial:      range_dict(50)
	)
})

test("random_operations_large_test", () {
	run_many_random_tests(
		count:        3
		ops_per_test: 300
		key_space:    600
		initial:      range_dict(300)
	)
})

range_dict = (size) {
	int.range(from: size, to: 0, with: [], run: list.prepend)
		|> list.map((x) { (x, x) })
		|> dict.from_list
}

prng = (state: Int) -> Int { { state * 48_271 } % 0x7FFFFFFF }

assert_dict_matches_proplist(K, V) = (
	d: dict.Dict(K, V)
	proplist: List((K, V))
	seed: Int
) -> Nil {
	{ dict.size(d) == list.length(proplist) } |> {
		True -> Nil
		False -> panic(
			"Size mismatch with seed " <>
				int.to_string(seed) <>
				": dict.size=" <>
				int.to_string(dict.size(d)) <>
				" proplist.size=" <>
				int.to_string(list.length(proplist))
		)
	}
	list.each(proplist, (pair) {
		(key, value) = pair
		got = dict.get(d, key)
		{ got == Some(value) } |> {
			True -> Nil
			False -> panic(
				"Get mismatch with seed " <>
					int.to_string(seed) <>
					": key=" <>
					string.inspect(key) <>
					", value=" <>
					string.inspect(value) <>
					", dict.get=" <>
					string.inspect(got)
			)
		}
	})
	{ d == dict.from_list(proplist) } |> {
		True -> Nil
		False -> panic(
			"Structural equality failed with seed " <> int.to_string(seed)
		)
	}
}

// Tests for the hash trie, and for functions that are new in std.

test("get_option_test", () {
	d = dict.from_list([("a", 1), ("b", 2)])
	assert(dict.get(d, "a") == Some(1))
	assert(dict.get(d, "z") == None)
	assert(dict.get(dict.new(), 1) == None)
})

test("insert_replace_test", () {
	d = dict.new() |> dict.insert(1, "a") |> dict.insert(1, "b")
	assert(dict.size(d) == 1)
	assert(dict.get(d, 1) == Some("b"))
})

test("delete_missing_test", () {
	d = dict.from_list([(1, 1)])
	assert(dict.delete(d, 2) == d)
	assert(dict.size(dict.delete(d, 2)) == 1)
	assert(dict.delete(d, 1) == dict.new())
	assert(dict.is_empty(dict.delete(d, 1)))
})

test("equality_any_order_test", () {
	xs = list.range(1, 2000)
	a = dict.from_list(list.map(xs, (i) { (i, i * 2) }))
	b = dict.from_list(list.map(list.reverse(xs), (i) { (i, i * 2) }))
	assert(a == b)
	// Deleting gives the same tree as never inserting.
	c = list.fold(list.range(2001, 3000), b, (d, i) { dict.insert(d, i, 0) })
	c2 = list.fold(list.range(2001, 3000), c, (d, i) { dict.delete(d, i) })
	assert(c2 == a)
})

test("many_keys_test", () {
	n = 100_000
	d = list.fold(list.range(1, n), dict.new(), (d, i) {
		dict.insert(d, i, i * i)
	})
	assert(dict.size(d) == n)
	assert(dict.get(d, 1) == Some(1))
	assert(dict.get(d, n) == Some(n * n))
	assert(dict.get(d, n + 1) == None)
	assert(list.all(list.range(1, n), (i) { dict.get(d, i) == Some(i * i) }))
	d2 = list.fold(list.range(1, n), d, (d, i) {
		i % 2 |> {
			0 -> dict.delete(d, i)
			_ -> d
		}
	})
	assert(dict.size(d2) == n / 2)
	assert(dict.get(d2, 2) == None)
	assert(dict.get(d2, 3) == Some(9))
	assert(
		list.sort(dict.keys(d2), int.compare) ==
			list.filter(list.range(1, n), int.is_odd)
	)
})

test("string_keys_test", () {
	keys = list.map(list.range(1, 5000), (i) { "key" <> int.to_string(i) })
	d = dict.from_list(list.map(keys, (k) { (k, string.length(k)) }))
	assert(dict.size(d) == 5000)
	assert(dict.get(d, "key4999") == Some(7))
	assert(dict.has(d, "key1"))
	assert(core.not(dict.has(d, "key0")))
})

Point {
	Point(x: Int, y: Int)
}

test("any_key_type_test", () {
	d = dict.new()
		|> dict.insert(Point(1, 2), "a")
		|> dict.insert((1, [2, 3]), "b")
		|> dict.insert(<<1, 2>>, "c")
		|> dict.insert(1.5, "d")
	assert(dict.size(d) == 4)
	assert(dict.get(d, Point(1, 2)) == Some("a"))
	assert(dict.get(d, (1, [2, 3])) == Some("b"))
	assert(dict.get(d, <<1, 2>>) == Some("c"))
	assert(dict.get(d, 1.5) == Some("d"))
	assert(dict.get(d, Point(2, 1)) == None)
})

test("zero_keys_test", () {
	// 0.0 and -0.0 are the same value, so they are one key.
	d = dict.new() |> dict.insert(0.0, "a") |> dict.insert(-.0.0, "b")
	assert(dict.size(d) == 1)
	assert(dict.get(d, 0.0) == Some("b"))
})

test("upsert_update_test", () {
	count = (words) {
		list.fold(words, dict.new(), (d, w) {
			dict.upsert(d, w, (x) {
				x |> {
					Some(n) -> n + 1
					None    -> 1
				}
			})
		})
	}
	d = count(["a", "b", "a", "c", "a"])
	assert(dict.get(d, "a") == Some(3))
	assert(dict.get(d, "c") == Some(1))
	assert(dict.update(d, "a", (n) { n * 10 }) |> dict.get("a") == Some(30))
	assert(dict.update(d, "z", (n) { n * 10 }) == d)
})

test("merge_combine_test", () {
	a = dict.from_list([(1, "a"), (2, "b")])
	b = dict.from_list([(2, "x"), (3, "y")])
	assert(dict.merge(a, b) == dict.from_list([(1, "a"), (2, "x"), (3, "y")]))
	assert(
		dict.combine(a, b, (x, y) { x <> y }) ==
			dict.from_list([(1, "a"), (2, "bx"), (3, "y")])
	)
})

test("group_dict_test", () {
	g = dict.group(["apple", "avocado", "banana", "cherry", "blueberry"], (s) {
		string.slice(s, 0, 1)
	})
	assert(dict.get(g, "a") == Some(["apple", "avocado"]))
	assert(dict.get(g, "b") == Some(["banana", "blueberry"]))
	assert(dict.get(g, "c") == Some(["cherry"]))
})

test("fold_effects_test", () {
	d = dict.from_list([(1, 1), (2, 2)])
	r = fail.catch(() { dict.fold(d, 0, (_, _, _) { fail.raise("stop") }) })
	assert(r == Error("stop"))
})

test("map_values_keys_test", () {
	d = dict.from_list([(1, 10), (2, 20)])
	assert(
		dict.map_values(d, (k, v) { k + v }) ==
			dict.from_list([(1, 11), (2, 22)])
	)
	assert(dict.map_values(dict.new(), (k, v) { k + v }) == dict.new())
})

test("filter_take_drop_test", () {
	d = dict.from_list(list.map(list.range(1, 10), (i) { (i, i) }))
	assert(dict.size(dict.filter(d, (_, v) { v > 5 })) == 5)
	assert(dict.take(d, [1, 2, 99]) == dict.from_list([(1, 1), (2, 2)]))
	assert(dict.size(dict.drop(d, [1, 2, 99])) == 8)
})
