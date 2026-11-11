// spec: 9.7, 10.3
// expect: compile-error E0320
@host("test:kv/v1")
Kv { Kv(get: (key: String) -> "a" | "b" ^ String) }

main = (kv ! Kv) { kv.get("name") |> { "a" -> Nil  "b" -> Nil } }
