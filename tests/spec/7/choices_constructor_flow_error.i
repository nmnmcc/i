// spec: 4.8, 6.12, 7.5
// expect: compile-error E0512
T { Yes(value: Int) No }
f = (r: T) { r |> { Yes(_) | Yes(_) -> r.value } }
