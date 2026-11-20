(
	testlib = (test) = i/test
	i/core
	int = i/int
	list = i/list
	string = i/string
	order = (Less, Equal, Greater) = i/order
)

test("negate_lt_test", () {
	assert(order.negate(Less()) == Greater())
	()
})

test("negate_eq_test", () {
	assert(order.negate(Equal()) == Equal())
	()
})

test("negate_gt_test", () {
	assert(order.negate(Greater()) == Less())
	()
})

test("to_int_lt_test", () {
	assert(order.to_int(Less()) == -1)
	()
})

test("to_int_eq_test", () {
	assert(order.to_int(Equal()) == 0)
	()
})

test("to_int_gt_test", () {
	assert(order.to_int(Greater()) == 1)
	()
})

test("compare_lt_lt_test", () {
	assert(order.compare(Less(), Less()) == Equal())
	()
})

test("compare_lt_eq_test", () {
	assert(order.compare(Less(), Equal()) == Less())
	()
})

test("compare_lt_gt_test", () {
	assert(order.compare(Less(), Greater()) == Less())
	()
})

test("compare_eq_lt_test", () {
	assert(order.compare(Equal(), Less()) == Greater())
	()
})

test("compare_eq_eq_test", () {
	assert(order.compare(Equal(), Equal()) == Equal())
	()
})

test("compare_eq_gt_test", () {
	assert(order.compare(Equal(), Greater()) == Less())
	()
})

test("compare_gt_lt_test", () {
	assert(order.compare(Greater(), Less()) == Greater())
	()
})

test("compare_gt_eq_test", () {
	assert(order.compare(Greater(), Equal()) == Greater())
	()
})

test("compare_gt_gt_test", () {
	assert(order.compare(Greater(), Greater()) == Equal())
	()
})

test("reverse_test", () {
	assert(
		list.sort([4, 5, 1], order.reverse(int.compare)) == [5, 4, 1],
	)
	()
})

test("break_tie_eq_lt_test", () {
	assert(order.break_tie(Equal(), Less()) == Less())
	()
})

test("break_tie_eq_gt_test", () {
	assert(order.break_tie(Equal(), Greater()) == Greater())
	()
})

test("break_tie_eq_eq_test", () {
	assert(order.break_tie(Equal(), Equal()) == Equal())
	()
})

test("break_tie_gt_lt_test", () {
	assert(order.break_tie(Greater(), Less()) == Greater())
	()
})

test("break_tie_lt_gt_test", () {
	assert(order.break_tie(Less(), Greater()) == Less())
	()
})

test("lazy_break_tie_eq_lt_test", () {
	assert(order.lazy_break_tie(Equal(), () { Less() }) == Less())
	()
})

test("lazy_break_tie_eq_gt_test", () {
	assert(order.lazy_break_tie(Equal(), () { Greater() }) == Greater())
	()
})

test("lazy_break_tie_eq_eq_test", () {
	assert(order.lazy_break_tie(Equal(), () { Equal() }) == Equal())
	()
})

test("lazy_break_tie_gt_lazy_test", () {
	assert(
		order.lazy_break_tie(Greater(), () { panic("unreachable") }) ==
			Greater(),
	)
	()
})

test("lazy_break_tie_lt_lazy_test", () {
	assert(order.lazy_break_tie(Less(), () { panic("unreachable") }) == Less())
	()
})

test("min_max_test", () {
	assert(order.max(Less(), Greater()) == Greater())
	assert(order.max(Equal(), Less()) == Equal())
	assert(order.min(Less(), Greater()) == Less())
	assert(order.min(Greater(), Equal()) == Equal())
})

test("is_test", () {
	assert(order.is_less(Less()))
	assert(
		core.not(order.is_less(Equal())),
	)
	assert(order.is_equal(Equal()))
	assert(order.is_greater(Greater()))
	assert(
		core.not(order.is_greater(Less())),
	)
})

test("reverse_sort_test", () {
	assert(
		list.sort([2, 3, 1], order.reverse(int.compare)) == [3, 2, 1],
	)
})

test("break_tie_sort_test", () {
	people = [("b", 2), ("a", 2), ("c", 1)]
	by_age_then_name = (x : (String, Int), y : (String, Int)) {
		order.break_tie(int.compare(x.1, y.1), string.compare(x.0, y.0))
	}
	assert(
		list.sort(people, by_age_then_name) == [("c", 1), ("a", 2), ("b", 2)],
	)
})
