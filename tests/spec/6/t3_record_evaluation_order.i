// spec: 6.2, 6.7
// expect: stop "panic: first"
// at: spec/t3_record_evaluation_order:7:7
(main)
@main
main = () {
  (b: panic("first"), a: panic("second"))
  Nil
}
