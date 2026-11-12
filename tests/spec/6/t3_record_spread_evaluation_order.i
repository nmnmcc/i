// spec: 6.2, 6.7
// expect: stop "panic: base"
// at: spec/main:7:6
(main)

base = () -> (a: Int, b: Int) { panic("base") }

main = () {
  (..base(), a: panic("field"))
  Nil
}
