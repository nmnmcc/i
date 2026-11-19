//// Build-only probe: the same sort and map code paths as the list_sort and
//// list_map benchmarks, so the optimized module can be counted.
(
	int = std/int
	list = std/list
)

size = 1_000_000

randoms = (n : Int) -> List(Int) {
	go = (i, x, acc) {
		i |>
			(0) { acc }
			& (_) {
				y = x * 75 + 74
				go(i - 1, y % 65537, [x, ..acc])
			}
	}
	go(n, 1, [])
}

main = () {
	xs = list.sort(randoms(size), int.compare)
	list.length(xs)
}
