// §4.7: a type that has itself inside it is a compile error.
self_apply = (x) { x(x) }
local = () {
  f = (x) { x(x) }
  1
}
// Not recursive with subtypes: `x` is a value and a list of such values.
fine = (x) { [x, ..x] }
// Also when the type is not generalized, or not bound to a name; each is
// reported once.
id = (x) { x }
mono = () {
  h = id((x) { x(x) })
  1
}
unbound = () {
  id((x) { x(x) })
  1
}
