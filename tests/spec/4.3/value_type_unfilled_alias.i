// spec: 3, 4.3, 4.7
// expect: compile-error E0534
id = (x) { x }

value = id(id)

Alias = value
