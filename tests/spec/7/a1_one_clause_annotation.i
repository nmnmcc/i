// spec: opening, 6.12, 7.1
// expect: compile-error E0501
(main)
read = (v: Int | String) { v |> { x: Int -> x } }


main = () {
  read("other")
  Nil
}
