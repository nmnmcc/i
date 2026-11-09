// §4.7: a type that has itself inside it is a compile error.
self_apply = (x) { x(x) }
local = () {
  f = (x) { x(x) }
  1
}
// Not recursive with subtypes: `x` is a value and a list of such values.
fine = (x) { [x, ..x] }
