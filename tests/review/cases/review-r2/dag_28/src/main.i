(
	i/task
	(Io) = i/io
)
Leaf = ()
Node = (Tree, Tree)
Tree = Leaf | Node
make = (n : Int) -> Tree {
	n |>
		(0) { Leaf }
		| (_) {
			x = make(n - 1)
			Node(x, x)
		}
}

main = () <- (io : Io) {
	x = make(28)
	task.scope((s) {
		t = task.go(s, () { x |> (Leaf) { 0 } | (Node(_, _)) { 1 } })
		assert(task.join(t) == 1)
	})
	io.print("dag-ok")
}
