// spec: 6.2, 6.7
// expect: stop "panic: base"
// at: spec/t3_record_spread_evaluation_order:7:6
(main)

main = () {
  (..panic("base"), a: panic("field"))
  Nil
}
