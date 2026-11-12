// spec: 4.8, 6.12
// expect: pass
(main)
User { User(name: String) }
Option(A) { Some(A) None }
list_map(A, B) = (xs: List(A), f: (A) -> B) -> List(B) {
  xs |> {
    [] -> []
    [x, ..rest] -> [f(x), ..list_map(rest, f)]
  }
}


main = () {
  opts: List(Option(User)) = [Some(User(name: "Ada")), None, Some(User(name: "Bob"))]
  names = list_map(opts, { Some(u) -> u.name  None -> "" })
  assert(names == ["Ada", "", "Bob"])
  Nil
}
