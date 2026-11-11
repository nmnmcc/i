// spec: 3, 4.9
// expect: compile-error E0540
(main)
@main
main = () {
  value: (name: Int, name: String) = (name: 1)
  value
}
