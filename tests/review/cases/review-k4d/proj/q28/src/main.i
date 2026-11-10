S(N) { S(f: () -> Int ! N) }
use(N) = (s ! S(N)) {
  S(f) = s
  f()
}
fw(M) = () -> Int ! M | S(M) { use() }
