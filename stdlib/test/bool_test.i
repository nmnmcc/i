(
	testlib = (test) = i/test
	i/core
	bool = i/bool
	order = (Less, Equal, Greater) = i/order
)

test("and_true_true_test", () {
	assert(bool.and(True(), True()))
	()
})

test("and_false_true_test", () {
	assert(
		core.not(bool.and(False(), True())),
	)
	()
})

test("and_true_true_2_test", () {
	assert(bool.and(True(), True()))
	()
})

test("and_true_false_test", () {
	assert(
		core.not(bool.and(True(), False())),
	)
	()
})

test("or_true_true_test", () {
	assert(bool.or(True(), True()))
	()
})

test("or_false_true_test", () {
	assert(bool.or(False(), True()))
	()
})

test("or_true_false_test", () {
	assert(bool.or(True(), False()))
	()
})

test("or_true_false_2_test", () {
	assert(bool.or(True(), False()))
	()
})

test("negate_true_test", () {
	assert(
		core.not(bool.not(True())),
	)
	()
})

test("negate_false_test", () {
	assert(bool.not(False()))
	()
})

test("nor_false_false_test", () {
	assert(bool.nor(False(), False()))
	()
})

test("nor_false_true_test", () {
	assert(
		core.not(bool.nor(False(), True())),
	)
	()
})

test("nor_true_false_test", () {
	assert(
		core.not(bool.nor(True(), False())),
	)
	()
})

test("nor_true_true_test", () {
	assert(
		core.not(bool.nor(True(), True())),
	)
	()
})

test("nand_false_false_test", () {
	assert(bool.nand(False(), False()))
	()
})

test("nand_false_true_test", () {
	assert(bool.nand(False(), True()))
	()
})

test("nand_true_false_test", () {
	assert(bool.nand(True(), False()))
	()
})

test("nand_true_true_test", () {
	assert(
		core.not(bool.nand(True(), True())),
	)
	()
})

test("exclusive_or_true_true_test", () {
	assert(
		core.not(bool.xor(True(), True())),
	)
	()
})

test("exclusive_or_false_false_test", () {
	assert(
		core.not(bool.xor(False(), False())),
	)
	()
})

test("exclusive_or_true_false_test", () {
	assert(bool.xor(True(), False()))
	()
})

test("exclusive_or_false_true_test", () {
	assert(bool.xor(False(), True()))
	()
})

test("exclusive_nor_false_false_test", () {
	assert(bool.exclusive_nor(False(), False()))
	()
})

test("exclusive_nor_false_true_test", () {
	assert(
		core.not(bool.exclusive_nor(False(), True())),
	)
	()
})

test("exclusive_nor_true_false_test", () {
	assert(
		core.not(bool.exclusive_nor(True(), False())),
	)
	()
})

test("exclusive_nor_true_true_test", () {
	assert(bool.exclusive_nor(True(), True()))
	()
})

test("to_string_true_test", () {
	assert(bool.to_string(True()) == "True")
	()
})

test("to_string_false_test", () {
	assert(bool.to_string(False()) == "False")
	()
})

test("guard_when_true_test", () { assert(bool.guard(True(), 2, () { 1 }) == 2) })

test("guard_when_false_test", () {
	assert(bool.guard(False(), 2, () { 1 }) == 1)
})

test("lazy_guard_when_true_test", () {
	oops = () { panic("this shouldn't run!") }
	assert(bool.lazy_guard(True(), () { 2 }, oops) == 2)
})

test("lazy_guard_when_false_test", () {
	oops = () { panic("this shouldn't run!") }
	assert(bool.lazy_guard(False(), oops, () { 1 }) == 1)
})

test("compare_test", () {
	assert(bool.compare(False(), True()) == Less())
	assert(bool.compare(True(), True()) == Equal())
	assert(bool.compare(True(), False()) == Greater())
})

test("to_int_test", () {
	assert(bool.to_int(True()) == 1)
	assert(bool.to_int(False()) == 0)
})
