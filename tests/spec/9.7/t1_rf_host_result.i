// spec: 9.7, 10.3
// expect: compile-error E0320
@host("test:kv/v1")
Kv = (get = (String) <- (Fail(String)) -> "a" | "b")

main = () <- (kv: Kv) { kv.get("name") |> ("a") { Nil } | ("b") { Nil } }
