id = (x) { x }
v = id(id)
f = () {
	copy = v
	g: copy = (x: Int) { x }
	g
}
G = f
