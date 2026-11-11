// spec: 4.5, 9.8
// expect: compile-error E0501
// multicore.md D13: Key(Int) cannot be used as Key(Int | String).
(i/key)

wide = (k: Key(Int | String)) { key.wrap(k, "x") }

main = () {
  narrow: Key(Int) = key.new()
  wide(narrow)
  Nil
}
