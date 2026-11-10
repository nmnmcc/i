(assertion = i/assertion)
pair = ("b", () { ("a",_) = pair  1 })
@main
main = () { assertion.catch(pair.1) |> { Error(_) -> Nil  Ok(_) -> panic("missed Assert") } }
