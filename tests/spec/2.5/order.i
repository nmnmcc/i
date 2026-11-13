// spec: 2.5
// expect: pass
(
	testlib = (test) = std/test
	i/core
)

check = (got: String, want: String) {
	{ got == want } |>
		(True) { Nil }
		| (False) { panic(got <> " is not " <> want) }
}

// `total` needs `double` and `base`, which come later in the source.
total = double(base) + offset

double = (n) { n * 2 + offset }

offset = 1

base = 20

// A direct value only reads the names outside its function bodies.
table = [(n) { n + later }, (n) { n * later }]

later = 10

(first, second) = (base + 1, "two")

apply_all = (fs, x) {
	fs |> ([]) { [] } | ([f, ..rest]) { [f(x), ..apply_all(rest, x)] }
}

test("values", () {
	check(core.inspect(total), "42")
	check(
		core.inspect(apply_all(table, 3)),
		"[13, 30]",
	)
	check(core.inspect((first, second)), "(21, \"two\")")
})
