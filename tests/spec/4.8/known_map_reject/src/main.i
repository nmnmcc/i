// spec: 4.8
// expect: compile-error E0520

(list = std/list)
x = list.map(["a"], (s) { s == "b" })
