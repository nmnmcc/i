(
	g = app/types
)

f = () {
	{
		g = (x) -> Int {
			g.Box(x) |> {
				g.Box(v) -> v
			}
		}
		{
			g(42) |> {
				_ -> 1
			}
		}
	} *
		10
}
