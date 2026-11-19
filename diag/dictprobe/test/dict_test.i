(
	testlib = (test) = std/test
	i/core
	dict = std/dict
	mydict = dictprobe/mydict
	list = std/list
)

size = 200_000

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

check = (ok : Bool, what : String) -> () {
	ok |> (True) { () } & (False) { panic(what) }
}

test("diag_std_insert", () {
	d = list.fold(randoms(size), dict.new(), (d, x) {
		dict.insert(d, x, x + 1)
	})
	check(dict.size(d) > 0, "size")
})

test("diag_local_insert", () {
	d = list.fold(randoms(size), mydict.new(), (d, x) {
		mydict.insert(d, x, x + 1)
	})
	check(mydict.size(d) > 0, "size")
})
