//// A function body that is one block containing another block. The inner
//// block runs where it is written, keeps its own bindings, and does not leak
//// them to the body. `i run` prints the lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
)

f = (x : Int) -> Int {
	{
		a = x + 1
		b = a + 1
		b
	}
}

main = () <- (io : Io) {
	io.print(core.inspect(f(1)))
	{
		{
			io.print("inner")
		}
	}
	io.print("outer")
}
