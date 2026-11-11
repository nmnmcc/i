// spec: 4.9
// expect: compile-error E0542
(i/record)
(main)
@main
main = () {
  get = record.get
  Nil
}
