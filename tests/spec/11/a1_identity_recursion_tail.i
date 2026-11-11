// spec: 6.10, 6.12, 11.4, 11.6
// expect: pass
(main)
make = () { { n -> n } }
loop = { 0 -> 0  n -> loop(n - 1) }
via_match = (n) {
  n |> { 0 -> 0  _ -> via_match(n - 1) }
}


main = () {
  first = make()
  second = make()
  assert(first == first)
  assert(first != second)
  assert(first(4) == 4)
  assert(loop(1000000) == 0)
  local = { 0 -> 0  n -> local(n - 1) }
  assert(local(1000000) == 0)
  assert(via_match(1000000) == 0)
  Nil
}
