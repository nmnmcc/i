// spec: 4.9, 6.7
// expect: pass
(list = std/list, i/record)
(main)

main = () {
  r = list.fold([1, 2, 3], (n: 0, total: 0), (acc: (n: Int, total: Int), x) {
    (..acc, n: acc.n + 1, total: acc.total + x)
  })
  s = list.fold([1, 2, 3], (n: 0), (acc, _) {
    { acc.n == 3 } |> {
      True -> acc
      False -> (n: acc.n + 1)
    }
  })
  assert(r == (n: 3, total: 6))
  assert(s.n == 3)
  assert(record.size(r) == 2)
  Nil
}
