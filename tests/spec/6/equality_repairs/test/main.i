// spec: 4.2, 6.9
// expect: warning W0503
(
	list = std/list
)

// W520: exact literal types allow ordinary runtime comparisons.
status = () { "ok" }

main = () {
	x = 1
	assert({ x == 2 } == False)
	assert({ status() == "error" } == False)
	assert(list.map(["a"], (x) { x == "b" }) == [False])
}
