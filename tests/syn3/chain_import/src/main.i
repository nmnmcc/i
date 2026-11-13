//// Syntax three, stages 9 and 11 (docs/design/syntax-3.md §2.6.2, §2.6.7): a
//// chain of another module is called directly as a member, as an imported
//// name and through a direct alias. `i run` prints
//// the lines of `expected.txt`.
(
	(Io) = i/io
	core = i/core
	geo = (area) = syn3_chain_import/geo
)

mine = geo.area

s: String = geo.show(1)

alias = geo.show

n: Int = alias("x")

main = () <- (io: Io) {
	io.print(
		core.inspect(geo.area(2.0)),
	)
	io.print(
		core.inspect(area(2.0, 3.0)),
	)
	io.print(
		core.inspect(mine(1.0)),
	)
	io.print(
		core.inspect(4.0 |> geo.area(1.0)),
	)
	io.print(s)
	io.print(core.inspect(n))
}
