(
	testlib = (test) = std/test
	path = std/path
	list = std/list
	string = std/string
)

// Expected spellings in these tables are the examples and rules of std-1 §5.
test("path_clean_examples", () {
	list.each([
		("", "."),
		("a//b/../", "a"),
		("/../../a", "/a"),
		("a/../../b", "../b"),
		("//server///x", "/server/x"),
		("C:\\a\\b", "C:\\a\\b"),
		("%2e%2e/x", "%2e%2e/x"),
		("././", "."),
		("../../..", "../../.."),
		("///", "/"),
		("é/../猫", "猫"),
		("é/É", "é/É"),
		("~/a", "~/a"),
		("a\u{0}/b", "a\u{0}/b"),
		("A/a", "A/a"),
	], (pair) {
		(input, expected) = pair
		assert(path.clean(input) == expected)
	})
	assert(path.join([]) == "")
	assert(path.join(["", ""]) == "")
	assert(path.join(["a", "/b", "..", "c"]) == "a/c")
	assert(path.join(["", "/", "", "a"]) == "/a")
	Nil
})

test("path_parts_examples", () {
	assert(path.split("a//b/") == ("a//b/", ""))
	assert(path.split("a//b") == ("a//", "b"))
	assert(path.split("") == ("", ""))
	assert(path.base("a/../") == "..")
	assert(path.base("///") == "/")
	assert(path.base("") == ".")
	assert(path.dir("a/b/") == "a/b")
	assert(path.dir("a") == ".")
	assert(path.extension(".profile") == ".profile")
	assert(path.extension("a.") == ".")
	assert(path.extension("a.txt/") == "")
	assert(path.extension("a.b.c") == ".c")
	assert(path.extension("a.b/c") == "")
	assert(path.extension("é.猫") == ".猫")
	assert(path.is_absolute("/a"))
	assert(path.is_absolute("//"))
	assert(path.is_absolute("C:/a") == False)
	assert(path.is_absolute("") == False)
	Nil
})

test("path_relative_examples", () {
	assert(path.relative("/a/b", "/a/c") == Ok("../c"))
	assert(path.relative("../a", "../b") == Ok("../b"))
	assert(path.relative("..", "../a") == Ok("a"))
	assert(path.relative("..", ".") == Error(path.UnknownParent))
	assert(path.relative("a", "/a") == Error(path.DifferentRoots))
	assert(path.relative("/a", "a") == Error(path.DifferentRoots))
	assert(path.relative("/a/./", "/a") == Ok("."))
	assert(path.relative("", "") == Ok("."))
	assert(path.relative(".", "../é") == Ok("../é"))
	assert(path.relative("../../a", "../a") == Error(path.UnknownParent))
	Nil
})

short_paths = (depth : Int) -> List(String) {
	depth |>
		(0) { [""] }
		| (_) {
			[
				"",
				..list.flat_map(short_paths(depth - 1), (prefix) {
					list.map(["/", ".", "a", "é", "\\"], (c) { prefix <> c })
				}),
			]
		}
}

// §5.3: clean idempotence, exact raw split, and the successful relative law.
test("path_exhaustive_laws", () {
	paths = short_paths(4)
	list.each(paths, (p) {
		assert(
			path.clean(path.clean(p)) == path.clean(p),
		)
		(a, b) = path.split(p)
		assert(a <> b == p)
	})
	list.each(short_paths(3), (from) {
		list.each(short_paths(3), (to) {
			path.relative(from, to) |>
				(Error(_)) { Nil }
				| (Ok(r)) {
					assert(
						path.clean(
							path.join([path.clean(from), r]),
						) ==
							path.clean(to),
					)
				}
		})
	})
	Nil
})

// §5.3 calls for 100,000 segments, multibyte names, and long dot/parent runs.
test("path_long_inputs", () {
	names = list.repeat("é猫", 100000)
	long = string.join(names, "/")
	assert(path.clean(long) == long)
	assert(path.join(names) == long)
	assert(path.base(long) == "é猫")
	assert(
		path.split(long) ==
			(string.join(list.repeat("é猫", 99999), "/") <> "/", "é猫"),
	)
	assert(
		path.clean(
			string.join(list.repeat(".", 100000), "/"),
		) ==
			".",
	)
	parents = string.join(list.repeat("..", 100000), "/")
	assert(path.clean(parents) == parents)
	assert(path.clean("/" <> parents) == "/")
	assert(
		path.clean(path.join([long, parents])) == ".",
	)
	name = string.repeat("é", 100000)
	assert(path.clean(name) == name)
	assert(path.split(name) == ("", name))
	assert(path.relative(long, long) == Ok("."))
	Nil
})
