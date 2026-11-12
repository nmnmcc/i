// spec: 4.9, 20.3
// expect: pass
(
	main
)
choose = (b: Bool) { b |> (True) { (a = 1, b = 2) } | (False) { (a = "x") } }

main = () {
	assert(choose(True).a == 1)
	assert(choose(False).a == "x")
	Nil
}
