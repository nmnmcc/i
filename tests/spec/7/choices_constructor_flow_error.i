// spec: 4.8, 6.12, 7.5
// expect: compile-error E0512
Yes = (value = Int)
No = ()
T = Yes | No
f = (r : T) { r |> (Yes(_) | Yes(_)) { r.value } }
