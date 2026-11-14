(
	testlib = (test) = std/test
	i/core
	i/fail
	i/control
	std/string
	std/list
	(Some, None) = std/option
)

// The fixed cases below are S1–S7 and the independent review's string table.
test("access_and_taking", () {
	assert(string.at("aé😀", 1) == Some("é"))
	assert(string.at("aé😀", -1) == Some("😀"))
	assert(string.at("aé😀", -3) == Some("a"))
	assert(string.at("aé😀", -4) == None)
	assert(string.at("aé😀", 3) == None)
	assert(string.at("", 0) == None)
	assert(string.at("e\u{301}", 1) == Some("\u{301}"))
	assert(string.at("👩\u{200d}💻", 1) == Some("\u{200d}"))
	assert(string.take_start("aé😀", -1) == "")
	assert(string.take_start("aé😀", 2) == "aé")
	assert(string.take_start("aé😀", 20) == "aé😀")
	assert(string.take_end("aé😀", -1) == "")
	assert(string.take_end("aé😀", 2) == "é😀")
	assert(string.take_end("aé😀", 20) == "aé😀")
	assert(string.take_end("", 2) == "")
	list.each(["", "abc", "é😀e\u{301}", "\u{0}"], (s) {
		n = string.length(s)
		list.each(list.range(0, n + 2), (k) {
			assert(string.take_start(s, k) <> string.drop_start(s, k) == s)
			assert(string.drop_end(s, k) <> string.take_end(s, k) == s)
		})
		list.each(list.range(-n - 1, n + 1), (i) {
			normalized = { i < 0 } |> (True) { n + i } | (False) { i }
			{ normalized >= 0 && normalized < n } |>
				(True) {
					assert(string.at(s, i) == Some(string.slice(s, i, 1)))
				}
				| (False) { assert(string.at(s, i) == None) }
		})
	})
})

test("search_bounds_and_overlaps", () {
	assert(string.index_of("ééx", "x", 0) == Some(2))
	assert(string.last_index_of("aaa", "aa", 3) == Some(1))
	assert(string.last_index_of("aaa", "aa", 0) == Some(0))
	assert(string.last_index_of("aaa", "aa", 1) == Some(1))
	assert(string.index_of("aaa", "aa", 2) == None)
	assert(string.index_of("aé😀é", "é", 2) == Some(3))
	assert(string.last_index_of("aé😀é", "é", 2) == Some(1))
	list.each(["", "aé😀"], (s) {
		n = string.length(s)
		list.each(list.range(0, n), (i) {
			assert(string.index_of(s, "", i) == Some(i))
			assert(string.last_index_of(s, "", i) == Some(i))
		})
		list.each([-1, n + 1], (i) {
			assert(string.index_of(s, "", i) == None)
			assert(string.last_index_of(s, "", i) == None)
			assert(string.index_of(s, "a", i) == None)
			assert(string.last_index_of(s, "a", i) == None)
		})
		assert(string.index_of(s, "longer than the haystack", 0) == None)
	})
})

test("count_replacement_and_last_split", () {
	assert(string.count("aaaa", "aa") == 2)
	assert(string.count("aaa", "aa") == 1)
	assert(string.count("abc", "") == 0)
	assert(string.count("", "x") == 0)
	assert(string.replace_n("aaa", "a", "aa", 2) == "aaaaa")
	assert(string.replace_n("aaaaa", "aa", "x", 1) == "xaaa")
	assert(string.replace_n("abc", "", "x", 2) == "abc")
	assert(string.replace_n("abc", "a", "x", -1) == "abc")
	assert(string.split_once_last("a=b=c", "=") == Some(("a=b", "c")))
	assert(string.split_once_last("aaa", "aa") == Some(("a", "")))
	assert(string.split_once_last("=x=", "=") == Some(("=x", "")))
	assert(string.split_once_last("=x", "=") == Some(("", "x")))
	assert(string.split_once_last("x", "") == None)
	assert(string.split_once_last("x", "y") == None)
	list.each(["", "abc", "éé😀é", "aaaaa"], (s) {
		list.each(["", "a", "aa", "é", "😀"], (needle) {
			assert(
				string.replace_n(
					s,
					needle,
					"aaé",
					string.count(s, needle) + 1,
				) ==
					string.replace(s, needle, "aaé"),
			)
			string.split_once_last(s, needle) |>
				(None) { Nil }
				| (Some((left, right))) { assert(left <> needle <> right == s) }
		})
	})
})

test("scalar_traversals", () {
	assert(string.words(" a\t\n b ") == ["a", "b"])
	assert(string.words("a\u{a0}b") == ["a\u{a0}b"])
	assert(
		string.words("\u{85}a\u{200e}b\u{200f}c\u{2028}d\u{2029}") ==
			["a", "b", "c", "d"],
	)
	assert(
		string.split_when(",,a,,,b,", (c) { core.codepoint_to_int(c) == 44 }) ==
			["a", "b"],
	)
	assert(string.split_when("", (_) { panic("empty split callback") }) == [])
	assert(string.map("", (_) { panic("empty map callback") }) == "")
	assert(string.filter("", (_) { panic("empty filter callback") }) == "")
	assert(string.fold("", 7, (_, _) { panic("empty fold callback") }) == 7)
	assert(string.map("aé😀", (c) { c }) == "aé😀")
	assert(string.filter("aé😀", (c) { core.codepoint_to_int(c) > 127 }) == "é😀")
	assert(
		string.fold("aé😀", [], (acc, c) {
			list.append(acc, [core.codepoint_to_int(c)])
		}) ==
			[97, 233, 128512],
	)
})

Calls = (call = (Int) -> Nil)

observe = (body : () <- (Calls, Assert) -> Nil) -> List(Int) {
	control.boundary((b) {
		{
			body()
			[]
		} <- (
			Calls(
				call = (n) {
					control.suspend(b, (k) { [n, ..control.resume(k, Nil)] })
				},
			),
		)
	})
}

test("callbacks_run_once_in_order_and_stop_on_error", () {
	run = (
		action : ((UtfCodepoint) <- (Fail(String), Calls) -> UtfCodepoint) <- (
			Fail(String),
			Calls,
		) -> String
	) -> Nil {
		seen = observe(() {
			result = fail.catch(() {
				action((c) <- (calls : Calls) {
					n = core.codepoint_to_int(c)
					calls.call(n)
					{ n == 233 } |>
						(True) { fail.raise("second") }
						| (False) { c }
				})
			})
			assert(result == Error("second"))
		})
		assert(seen == [97, 233])
	}
	run((callback) { string.map("aé😀", callback) })
	run((callback) { string.filter("aé😀", (c) { callback(c) == c }) })
	run((callback) {
		string.join(string.split_when("aé😀", (c) { callback(c) != c }), "")
	})
	run((callback) {
		string.fold("aé😀", "", (acc, c) {
			acc <> core.from_codepoints([callback(c)])
		})
	})
	seen = observe(() {
		assert(
			string.map("aé😀", (c) <- (calls : Calls) {
				calls.call(core.codepoint_to_int(c))
				c
			}) ==
				"aé😀",
		)
	})
	assert(seen == [97, 233, 128512])
})
