// spec: 4.8
// expect: pass

(
    list = std/list,
    testlib = (test) = std/test
)

Box(A) = (value = A)

repeat = <S>(start: S, step: (S) -> S) -> S { step(step(start)) }

// The bounds imposed by these accesses constrain every accumulator value.
record = () { list.fold([1, 2], (n = 0), (acc, x) { (n = acc.n + x) }).n }
data = () { list.fold([1, 2], (0, 0), (acc, x) { (acc.0 + x, acc.1) }).0 }
nominal = () { list.fold([1, 2], Box(value = 0), (acc, x) { Box(value = acc.value + x) }).value }

// Hints propagated through aliases must stay nonfactual.
compare = () {
    repeat(0, (acc) {
        alias = acc
        _ = alias == "s"
        "s"
    }) == "s"
}

test("F1 access hints", () {
    assert(record() == 3)
    assert(data() == 3)
    assert(nominal() == 3)
    assert(compare())
})
