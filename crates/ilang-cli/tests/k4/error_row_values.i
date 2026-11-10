(i/fail)
one = () -> Nil ? 1 { fail.raise(1) }
two = () -> Nil ? 2 { fail.raise(2) }
pick = (b: Bool) { b |> { True -> one False -> two } }
@main
main = () {
  fail.catch(() { pick(True)() }) |> { Error(1) -> Nil _ -> assert(False) }
  fail.catch(() { pick(False)() }) |> { Error(2) -> Nil _ -> assert(False) }
}
