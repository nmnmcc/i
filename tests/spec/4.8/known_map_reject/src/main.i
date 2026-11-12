// spec: 4.8
// expect: warning W0503
(
	list = std/list
)
x = list.map(["a"], (s) { s == "b" })

main = () { assert(x == [False]) }
