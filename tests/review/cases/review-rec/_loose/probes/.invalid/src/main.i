// Predictions from the proposed rules; record syntax is not implemented here.
wrap = (x: Int | String) -> { a: Int | String } { a: x }
wrap_union = (x: Int | String) -> { a: Int } | { a: String } { a: x }

// Tuple subtyping is absent from U3's exhaustive list of successful rules.
pair = () -> { data: (Int, Int) } | { error: String } { data: (1, 2) }
pair_annotated = () -> { data: (Int, Int) } | { error: String } {
  data: (Int, Int) = (1, 2)
  { data: data }
}

// A correct annotation equal to the claimed inferred result triggers U3.
// These two definitions assume an ordinary import of i/record.
choose(R) = (c: Bool, r: R & {..}) {
  c |> { True -> record.put(r, "a", 1)  False -> { b: 2 } }
}
choose_written(R) = (c: Bool, r: R & {..}) -> { ..R, a: 1 } | { b: 2 } {
  c |> { True -> record.put(r, "a", 1)  False -> { b: 2 } }
}
