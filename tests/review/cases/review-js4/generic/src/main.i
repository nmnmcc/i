Cell(A) = (value = A)
Left = (value = Int)
Right = (value = String)
ints = () { Cell(value = 7) }
strings = () { Cell(value = "seven") }
left = () <- (c : Cell(Int)) { Left(value = c.value) }
right = () <- (c : Cell(String)) { Right(value = c.value) }
use = () <- (l : Left, r : Right) { (l.value, r.value) }

main = () {
	l = { left() <- (ints()) }
	r = { right() <- (strings()) }
	assert(use() == (7, "seven")) <- (l, r)
}
