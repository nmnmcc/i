// spec: 4.8, 6.12, 7.5
// expect: compile-error E0512
T = Yes | No
Yes { Yes(value: Int) }
No { No }
f = (r: T) { r |> { Yes(_) | Yes(_) -> r.value } }
