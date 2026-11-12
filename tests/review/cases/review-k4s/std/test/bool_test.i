(
	testlib = (test) = std/test
	i/core
	bool = std/bool
	order = (Less, Equal, Greater) = std/order
)

test("and_true_true_test", () {
	assert(bool.and(True, True))
	Nil
})

test("and_false_true_test", () {
	assert(core.not(bool.and(False, True)))
	Nil
})

test("and_true_true_2_test", () {
	assert(bool.and(True, True))
	Nil
})

test("and_true_false_test", () {
	assert(core.not(bool.and(True, False)))
	Nil
})

test("or_true_true_test", () {
	assert(bool.or(True, True))
	Nil
})

test("or_false_true_test", () {
	assert(bool.or(False, True))
	Nil
})

test("or_true_false_test", () {
	assert(bool.or(True, False))
	Nil
})

test("or_true_false_2_test", () {
	assert(bool.or(True, False))
	Nil
})

test("negate_true_test", () {
	assert(core.not(bool.not(True)))
	Nil
})

test("negate_false_test", () {
	assert(bool.not(False))
	Nil
})

test("nor_false_false_test", () {
	assert(bool.nor(False, False))
	Nil
})

test("nor_false_true_test", () {
	assert(core.not(bool.nor(False, True)))
	Nil
})

test("nor_true_false_test", () {
	assert(core.not(bool.nor(True, False)))
	Nil
})

test("nor_true_true_test", () {
	assert(core.not(bool.nor(True, True)))
	Nil
})

test("nand_false_false_test", () {
	assert(bool.nand(False, False))
	Nil
})

test("nand_false_true_test", () {
	assert(bool.nand(False, True))
	Nil
})

test("nand_true_false_test", () {
	assert(bool.nand(True, False))
	Nil
})

test("nand_true_true_test", () {
	assert(core.not(bool.nand(True, True)))
	Nil
})

test("exclusive_or_true_true_test", () {
	assert(core.not(bool.xor(True, True)))
	Nil
})

test("exclusive_or_false_false_test", () {
	assert(core.not(bool.xor(False, False)))
	Nil
})

test("exclusive_or_true_false_test", () {
	assert(bool.xor(True, False))
	Nil
})

test("exclusive_or_false_true_test", () {
	assert(bool.xor(False, True))
	Nil
})

test("exclusive_nor_false_false_test", () {
	assert(bool.exclusive_nor(False, False))
	Nil
})

test("exclusive_nor_false_true_test", () {
	assert(core.not(bool.exclusive_nor(False, True)))
	Nil
})

test("exclusive_nor_true_false_test", () {
	assert(core.not(bool.exclusive_nor(True, False)))
	Nil
})

test("exclusive_nor_true_true_test", () {
	assert(bool.exclusive_nor(True, True))
	Nil
})

test("to_string_true_test", () {
	assert(bool.to_string(True) == "True")
	Nil
})

test("to_string_false_test", () {
	assert(bool.to_string(False) == "False")
	Nil
})

test("guard_when_true_test", () {
	assert(bool.guard(when: True, return: 2, otherwise: () { 1 }) == 2)
})

test("guard_when_false_test", () {
	assert(bool.guard(when: False, return: 2, otherwise: () { 1 }) == 1)
})

test("lazy_guard_when_true_test", () {
	oops = () { panic("this shouldn't run!") }
	assert(bool.lazy_guard(when: True, return: () { 2 }, otherwise: oops) == 2)
})

test("lazy_guard_when_false_test", () {
	oops = () { panic("this shouldn't run!") }
	assert(bool.lazy_guard(when: False, return: oops, otherwise: () { 1 }) == 1)
})

test("compare_test", () {
	assert(bool.compare(False, True) == Less)
	assert(bool.compare(True, True) == Equal)
	assert(bool.compare(True, False) == Greater)
})

test("to_int_test", () {
	assert(bool.to_int(True) == 1)
	assert(bool.to_int(False) == 0)
})
