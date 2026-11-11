// spec: 4.3
// expect: compile-error E0232
// A non-value RHS makes this a generic alias; `_` is not a parameter (T2 P1).
(main)
User { User }
Schema(User, _) = Int


main = () {
  assert(True)
  Nil
}
