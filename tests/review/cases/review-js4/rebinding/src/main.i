Target { Web Native }
read = (t ! Target) { t |> { Web -> 1 Native -> 2 } }
probe = (t ! Target) {
  alias = t
  captured = () { alias |> { Web -> 1 Native -> 2 } }
  ! Target = Native
  (captured(), read())
}

main = () {
  ! Target = Web
  assert(probe() == (1, 2))
}
