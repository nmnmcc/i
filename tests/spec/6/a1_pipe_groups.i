// spec: 6.9
// expect: pass
(main)
User { User(name: String) }

@main
main = () {
  assert(1 + 2 |> { 3 -> True  _ -> False })
  assert({ 2 == 2 } |> { True -> 7  False -> 8 } == 7)
  assert({ 1 |> { 0 -> 1  _ -> 2 } } * 10 == 20)
  assert(1 + { 2 |> { n -> n } } == 3)
  assert(-{ 3 |> { n -> n } } == -3)
  assert({ 1 |> { _ -> User("Ada") } }.name == "Ada")
  assert({ 1 |> { _ -> (x) { x + 1 } } }(4) == 5)
  Nil
}
