// spec: 4.8, 6.12, 7.3
// expect: compile-error E0501
// Syntax three: a function literal's `-> R` leaves its needs open (they are
// inferred); the written full type of the binding closes them, so the
// failing choice does not fit `() -> Int`.
s = "b"
f : () -> Int = () { s |> ("a" | "c") { 1 } }
