// old-syntax-check: E0101,E0101,E0101
// Proposed T3 plus landed T2. A plain user-defined type and constructor.
Schema(A) { Schema(decode: () -> A) }
make = (c: Bool) {
  c |> {
    True -> Schema(decode: () { a: 1 })
    False -> Schema(decode: () { a: "x" })
  }
}
schema = make(True)
Schema(Row) = schema
// Row denotes { a: 1 } | { a: "x" }.
read = (r: Row) { r.a }                 // U4: E0518
read_inferred = (r) { r.a }            // accepts schema.decode()
read_written = (r: { a: 1 | "x" }) { r.a }
