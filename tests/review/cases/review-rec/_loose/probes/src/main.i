(i/record)
// Equivalent record unions preserve scalar and tuple subtyping.
wrap = (x: Int | String) -> (a: Int | String) { (a: x) }
wrap_union = (x: Int | String) -> (a: Int) | (a: String) { (a: x) }

// Tuple subtyping applies beneath record fields.
pair = () -> (data: (Int, Int)) | (error: String) { (data: (1, 2)) }
pair_annotated = () -> (data: (Int, Int)) | (error: String) {
  data: (Int, Int) = (1, 2)
  (data: data)
}

// The written union requires an unestablished inclusion; U3 rejects it.
choose(R) = (c: Bool, r: R & Record) {
  c |> { True -> record.put(r, "a", 1)  False -> (b: 2) }
}
choose_written(R) = (c: Bool, r: R & Record) -> (..R, a: 1) | (b: 2) {
  c |> { True -> record.put(r, "a", 1)  False -> (b: 2) }
}
