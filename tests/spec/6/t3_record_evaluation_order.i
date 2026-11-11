// spec: 6.2, 6.7
// expect: stop "panic: first"
// at: spec/main:7:7
(main)
@main
main = () {
  (b: panic("first"), a: panic("second"))
  Nil
}
