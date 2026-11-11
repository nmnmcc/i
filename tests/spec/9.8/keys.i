// spec: 9.8
// expect: pass
// multicore.md D13: unwrap gives Found only for the same key.
// multicore.md 13: keyed equality compares the key and the stored value.
(i/key, i/core)

@main
main = () {
  a: Key(Int) = key.new()
  b: Key(Int) = key.new()
  x = key.wrap(a, 37)
  assert(a == a)
  assert(a != b)
  assert(key.unwrap(k: a, x: x) == key.Found(37))
  assert(key.unwrap(b, x) == key.Missing)
  assert(x == key.wrap(a, 37))
  assert(x != key.wrap(a, 38))
  assert(x != key.wrap(b, 37))
  assert(core.inspect(a) == "//key")
  assert(core.inspect(x) == "//keyed")
  assert(core.hash(a) == core.hash(b))
  assert(core.hash(x) == core.hash(key.wrap(a, 37)))
  assert(core.portable(a))
  assert(core.portable(x))
}
