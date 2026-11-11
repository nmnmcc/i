// spec: 1.7
// expect: pass
(main)
int_match = { 0 -> 10
  -1 -> 20
  _ -> 30 }
float_match = { 0.0 -> 10
  -.1.0 -> 20
  _ -> 30 }
pair_match = {
  (0, y) -> y
  (x, _) -> x
}


main = () {
  tuple = (
    2
    3
  )
  assert(tuple.0 + tuple.1 == 5)
  assert([2
    3] == [2, 3])
  value = 9 -
    4
  assert(value == 5)
  assert(int_match(-1) == 20)
  assert(float_match(-.1.0) == 20)
  assert(pair_match((3, 4)) == 3)
  assert(3
    |> { x -> x + 1 } == 4)
  Nil
}
