S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f = f) = s
  f()
}
fw = () { use() }
