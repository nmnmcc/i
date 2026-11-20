(
	testlib = (test) = i/test
	i/core
	set = i/set
	int = i/int
	list = i/list
	option = (Some, None) = i/option
)

test("size_empty_test", () {
	assert(
		set.size(set.new()) == 0,
	)
	()
})

test("size_with_elements_test", () {
	assert({ set.new() |> set.insert(1) |> set.insert(2) |> set.size } == 2)
	()
})

test("size_with_duplicates_test", () {
	assert(
		{
			set.new()
				|> set.insert(1)
				|> set.insert(1)
				|> set.insert(2)
				|> set.size
		} ==
			2,
	)
	()
})

test("is_empty_new_test", () {
	assert(
		set.is_empty(set.new()),
	)
	()
})

test("is_empty_with_element_test", () {
	assert(
		core.not(set.new() |> set.insert(1) |> set.is_empty()),
	)
	()
})

test("is_empty_after_delete_test", () {
	assert(set.new() |> set.insert(1) |> set.delete(1) |> set.is_empty())
	()
})

test("contains_present_test", () {
	assert(set.new() |> set.insert(1) |> set.contains(1))
	()
})

test("contains_absent_test", () {
	assert(
		core.not(
			set.contains(set.new(), 1),
		),
	)
	()
})

test("delete_test", () {
	assert(
		core.not(
			set.new() |> set.insert(1) |> set.delete(1) |> set.contains(1),
		),
	)
	()
})

test("to_list_test", () {
	assert(
		{
			set.new()
				|> set.insert(2)
				|> set.insert(3)
				|> set.insert(4)
				|> set.to_list
				|> list.sort(int.compare)
		} ==
			[2, 3, 4],
	)
	()
})

test("from_list_test", () {
	assert(
		{
			[1, 1, 2, 4, 3, 2]
				|> set.from_list
				|> set.to_list
				|> list.sort(int.compare)
		} ==
			[1, 2, 3, 4],
	)
	()
})

test("fold_test", () {
	assert(
		{ [1, 3, 9] |> set.from_list |> set.fold(0, (m, a) { m + a }) } == 13,
	)
	()
})

test("map_test", () {
	assert(
		{ [1, 2, 3, 4] |> set.from_list |> set.map(int.to_string) } ==
			set.from_list(["1", "2", "3", "4"]),
	)
	()
})

test("filter_test", () {
	assert(
		{
			[1, 4, 6, 3, 675, 44, 67]
				|> set.from_list()
				|> set.filter(int.is_even)
				|> set.to_list
				|> list.sort(int.compare)
		} ==
			[4, 6, 44],
	)
	()
})

test("take_test", () {
	assert(
		{ [1, 2, 3] |> set.from_list |> set.take([1, 3, 5]) } ==
			set.from_list([1, 3]),
	)
	()
})

test("drop_test", () {
	assert(
		{ ["a", "b", "c"] |> set.from_list |> set.drop(["a", "b", "d"]) } ==
			set.from_list(["c"]),
	)
	()
})

test("union_test", () {
	assert(
		{
			set.union(set.from_list([1, 2]), set.from_list([2, 3]))
				|> set.to_list
				|> list.sort(int.compare)
		} ==
			[1, 2, 3],
	)
	()
})

test("intersection_test", () {
	assert(
		set.to_list(
			set.intersection(set.from_list([1, 2]), set.from_list([2, 3])),
		) ==
			[2],
	)
	()
})

test("difference_test", () {
	assert(
		set.to_list(
			set.difference(set.from_list([1, 2]), set.from_list([2, 3, 4])),
		) ==
			[1],
	)
	()
})

test("is_subset_false_superset_test", () {
	assert(
		core.not(
			set.is_subset(set.from_list([1, 2, 3, 4]), set.from_list([2, 4])),
		),
	)
	()
})

test("is_subset_true_test", () {
	assert(
		set.is_subset(set.from_list([2, 4]), set.from_list([1, 2, 3, 4])),
	)
	()
})

test("is_subset_disjoint_test", () {
	assert(
		core.not(
			set.is_subset(set.from_list([1, 2, 3]), set.from_list([4, 5, 6])),
		),
	)
	()
})

test("is_subset_partial_overlap_test", () {
	assert(
		core.not(
			set.is_subset(set.from_list([1, 2]), set.from_list([2, 3, 4])),
		),
	)
	()
})

test("is_disjoint_true_test", () {
	assert(
		set.is_disjoint(set.from_list([1, 2, 3]), set.from_list([4, 5, 6])),
	)
	()
})

test("is_disjoint_false_test", () {
	assert(
		core.not(
			set.is_disjoint(set.from_list([1, 2]), set.from_list([2, 3, 4])),
		),
	)
	()
})

test("symmetric_difference_test", () {
	assert(
		set.symmetric_difference(
			set.from_list([1, 2, 3]),
			set.from_list([3, 4]),
		) ==
			set.from_list([1, 2, 4]),
	)
	()
})

test("each_test", () {
	assert(
		{
			[1, 2, 3]
				|> set.from_list
				|> set.each((member) {
					member |>
						(1 | 2 | 3) { () }
						& (_) { panic("unexpected value") }
				})
		} ==
			(),
	)
	()
})

test("set_equality_test", () {
	assert(set.from_list([1, 2, 3]) == set.from_list([3, 2, 1, 2]))
	assert(
		set.delete(set.from_list([1, 2]), 2) == set.from_list([1]),
	)
})

test("set_ops_test", () {
	a = set.from_list([1, 2, 3, 4])
	b = set.from_list([3, 4, 5])
	assert(set.union(a, b) == set.from_list([1, 2, 3, 4, 5]))
	assert(set.intersection(a, b) == set.from_list([3, 4]))
	assert(set.difference(a, b) == set.from_list([1, 2]))
	assert(set.symmetric_difference(a, b) == set.from_list([1, 2, 5]))
	assert(
		set.is_subset(set.from_list([3]), b),
	)
	assert(
		core.not(set.is_subset(a, b)),
	)
	assert(
		set.is_disjoint(set.from_list([1]), b),
	)
	assert(
		core.not(set.is_disjoint(a, b)),
	)
})

test("set_many_test", () {
	s = set.from_list(
		list.map(list.range(1, 50_000), (i) { i % 1000 }),
	)
	assert(set.size(s) == 1000)
	assert(set.contains(s, 999))
	assert(
		core.not(set.contains(s, 1000)),
	)
	assert(
		list.sort(set.to_list(s), int.compare) == list.range(0, 999),
	)
})

test("set_map_filter_test", () {
	s = set.from_list([1, 2, 3, 4])
	assert(set.map(s, (x) { x % 2 }) == set.from_list([0, 1]))
	assert(set.filter(s, int.is_even) == set.from_list([2, 4]))
	assert(set.fold(s, 0, int.add) == 10)
})

test("same_elements_test", () {
	one_two = set.from_list([1, 2])
	assert(
		set.same_elements(set.new(), set.new()),
	)
	assert(
		set.same_elements(one_two, set.from_list([2, 1])),
	)
	assert(
		set.same_elements(one_two, set.insert(one_two, 1)),
	)
	assert(
		core.not(
			set.same_elements(one_two, set.from_list([1, 3])),
		),
	)
	assert(
		core.not(
			set.same_elements(one_two, set.from_list([1])),
		),
	)
	assert(
		core.not(
			set.same_elements(set.from_list([1]), one_two),
		),
	)
	assert(
		core.not(
			set.same_elements(set.new(), one_two),
		),
	)
})

// "f3zx" and "0pad" have the same 32-bit `core.hash`; the set keeps them in
// the order they were put in, so `==` differs while `same_elements` does not.
test("same_elements_colliding_members_test", () {
	assert(core.hash("f3zx") == core.hash("0pad"))
	a = set.new() |> set.insert("f3zx") |> set.insert("0pad")
	b = set.new() |> set.insert("0pad") |> set.insert("f3zx")
	assert(a != b)
	assert(set.same_elements(a, b))
	assert(set.contains(a, "0pad") && set.contains(b, "f3zx"))
})
