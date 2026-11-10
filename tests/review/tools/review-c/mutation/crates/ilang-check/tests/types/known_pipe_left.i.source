// §4.8: the left side of `|>` is the first argument of the call, so it has
// the expected type of the first parameter. A function expression with a
// parameter with no written type is typed last, with the expected type that
// the other arguments give.
User { User(name: String, age: Int) }

apply = (f: (User) -> a, x: User) -> a { f(x) }
map = (xs: List(a), f: (a) -> b) -> List(b) {
  case xs {
    [] -> []
    [x, ..rest] -> [f(x), ..map(rest, f)]
  }
}
flip = (f: (a) -> b, xs: List(a)) -> List(b) { map(xs, f) }
u = User("bob", 3)

left_fn = (u) { u.name } |> apply(u)
left_fn_later = (u) { u.age } |> flip([User("ann", 4)])
// A tuple on the left: its parts have the parts of the expected type.
pair = (p: ((User) -> Int, Int)) { p }
left_tuple = ((u) { u.age }, 1) |> pair
// With no known type for the callee, there is no expected type.
unknown = (g) { (u) { u.name } |> g }
