// A record argument is scalarized at O2, introducing a join before the
// second suspension. Capturing there must retain both environment links.
(
	i/control
)
Outer = (get = () -> Int)
Inner = (get = () -> Int)
outer = () <- (s: Outer) { s.get() }
read = () <- (s: Inner) { s.get() }
Entry = (at = Int)
stamp = (entry: Entry) { Entry(..entry, at = read()) }
main = () {
	control.boundary((b) {
		{
			{
				n = outer() / 10
				entry = stamp(Entry(at = n))
				assert(entry.at == read())
				Nil
			} <- (
				Inner(
					get = () {
						control.suspend(b, (k) { control.resume(k, 7) })
					},
				),
			)
		} <- (Outer(get = () { 0 }))
	})
}
