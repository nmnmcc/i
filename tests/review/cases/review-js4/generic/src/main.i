Cell(A) { Cell(value: A) }
Left { Left(value: Int) }
Right { Right(value: String) }
ints = () { Cell(7) }
strings = () { Cell("seven") }
left = (c ! Cell(Int)) { Left(c.value) }
right = (c ! Cell(String)) { Right(c.value) }
use = (l ! Left, r ! Right) { (l.value, r.value) }


main = () {
  l = {
    ! Cell(Int) = ints()
    left()
  }
  r = {
    ! Cell(String) = strings()
    right()
  }
  ! Left = l
  ! Right = r
  assert(use() == (7, "seven"))
}
