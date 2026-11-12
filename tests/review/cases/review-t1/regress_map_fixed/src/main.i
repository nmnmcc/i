(
	list = std/list
)
f = () { list.map(["a"], (x: String) { x == "b" }) }
