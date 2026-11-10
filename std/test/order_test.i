(
	i/core
	int = std/int
	list = std/list
	string = std/string
	order = (Less, Equal, Greater) = std/order
)

@test
negate_lt_test = () {
	assert(order.negate(Less) == Greater)
	Nil
}

@test
negate_eq_test = () {
	assert(order.negate(Equal) == Equal)
	Nil
}

@test
negate_gt_test = () {
	assert(order.negate(Greater) == Less)
	Nil
}

@test
to_int_lt_test = () {
	assert(order.to_int(Less) == -1)
	Nil
}

@test
to_int_eq_test = () {
	assert(order.to_int(Equal) == 0)
	Nil
}

@test
to_int_gt_test = () {
	assert(order.to_int(Greater) == 1)
	Nil
}

@test
compare_lt_lt_test = () {
	assert(order.compare(Less, Less) == Equal)
	Nil
}

@test
compare_lt_eq_test = () {
	assert(order.compare(Less, Equal) == Less)
	Nil
}

@test
compare_lt_gt_test = () {
	assert(order.compare(Less, Greater) == Less)
	Nil
}

@test
compare_eq_lt_test = () {
	assert(order.compare(Equal, Less) == Greater)
	Nil
}

@test
compare_eq_eq_test = () {
	assert(order.compare(Equal, Equal) == Equal)
	Nil
}

@test
compare_eq_gt_test = () {
	assert(order.compare(Equal, Greater) == Less)
	Nil
}

@test
compare_gt_lt_test = () {
	assert(order.compare(Greater, Less) == Greater)
	Nil
}

@test
compare_gt_eq_test = () {
	assert(order.compare(Greater, Equal) == Greater)
	Nil
}

@test
compare_gt_gt_test = () {
	assert(order.compare(Greater, Greater) == Equal)
	Nil
}

@test
reverse_test = () {
	assert(list.sort([4, 5, 1], by: order.reverse(int.compare)) == [5, 4, 1])
	Nil
}

@test
break_tie_eq_lt_test = () {
	assert(order.break_tie(in: Equal, with: Less) == Less)
	Nil
}

@test
break_tie_eq_gt_test = () {
	assert(order.break_tie(in: Equal, with: Greater) == Greater)
	Nil
}

@test
break_tie_eq_eq_test = () {
	assert(order.break_tie(in: Equal, with: Equal) == Equal)
	Nil
}

@test
break_tie_gt_lt_test = () {
	assert(order.break_tie(in: Greater, with: Less) == Greater)
	Nil
}

@test
break_tie_lt_gt_test = () {
	assert(order.break_tie(in: Less, with: Greater) == Less)
	Nil
}

@test
lazy_break_tie_eq_lt_test = () {
	assert(order.lazy_break_tie(in: Equal, with: () { Less }) == Less)
	Nil
}

@test
lazy_break_tie_eq_gt_test = () {
	assert(order.lazy_break_tie(in: Equal, with: () { Greater }) == Greater)
	Nil
}

@test
lazy_break_tie_eq_eq_test = () {
	assert(order.lazy_break_tie(in: Equal, with: () { Equal }) == Equal)
	Nil
}

@test
lazy_break_tie_gt_lazy_test = () {
	assert(
		order.lazy_break_tie(in: Greater, with: () { panic("unreachable") }) ==
			Greater
	)
	Nil
}

@test
lazy_break_tie_lt_lazy_test = () {
	assert(
		order.lazy_break_tie(in: Less, with: () { panic("unreachable") }) ==
			Less
	)
	Nil
}

@test
min_max_test = () {
	assert(order.max(Less, Greater) == Greater)
	assert(order.max(Equal, Less) == Equal)
	assert(order.min(Less, Greater) == Less)
	assert(order.min(Greater, Equal) == Equal)
}

@test
is_test = () {
	assert(order.is_less(Less))
	assert(core.not(order.is_less(Equal)))
	assert(order.is_equal(Equal))
	assert(order.is_greater(Greater))
	assert(core.not(order.is_greater(Less)))
}

@test
reverse_sort_test = () {
	assert(list.sort([2, 3, 1], order.reverse(int.compare)) == [3, 2, 1])
}

@test
break_tie_sort_test = () {
	people = [("b", 2), ("a", 2), ("c", 1)]
	by_age_then_name = (x: (String, Int), y: (String, Int)) {
		order.break_tie(
			in:   int.compare(x.1, y.1)
			with: string.compare(x.0, y.0)
		)
	}
	assert(
		list.sort(people, by_age_then_name) == [("c", 1), ("a", 2), ("b", 2)]
	)
}
