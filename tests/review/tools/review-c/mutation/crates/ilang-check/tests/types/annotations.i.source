import (i/fail)

// §4.7: annotations are used in inference, and a type variable in an
// annotation may become a more exact type.
exact = (x: a) -> a { x + 1 }
general = (x: a) -> a { x }
holes = (x: List(_)) { x }
inner = (x: a) -> a {
  same: (a) -> a = (y) { y }
  same(x)
}
// Two local bindings next to each other have their own `a`.
apart = () {
  f: (a) -> a = (x) { x }
  g: (a) -> a = (x) { x }
  (f(1), g("s"))
}
typed_value: Int = 1
fn_type: (Int) -> Int = (x) { x }
union_arg = (x: Int | String) { x }
inter_arg = (x: a & Int) -> a { x }
list_union: List(Int) | List(String) = [1]
result_written = (r: Result(a, e)) -> a ? e {
  case r {
    Ok(v) -> v
    Error(e) -> fail.raise(e)
  }
}
diff_written = (x: Int | String | Nil) -> { Int | String | Nil } - String { 1 }
alias_use = (p: Point) { p }
Point = (Int, Int)
Pair(a) = (a, a)
pair_use = (p: Pair(String)) { p }

// The inferred type must agree with an annotation.
wrong: String = 1
wrong_ret = (x) -> String { x + 1 }

