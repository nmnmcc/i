// spec: 4.9
// expect: compile-error E0541
(i/record)
(main)
f = (r: (a: Int), key: String) {
  key |> { "a" -> record.get(r, key)  _ -> 0 }
}
@main
main = () { Nil }
