Yes = (value = Int)
No = ()
// spec: 4.8, 6.12, 7.5
// expect: compile-error E0512
T = Yes | No
f = (r: T) { r |> (Yes(_) | Yes(_)) { r.value } }
