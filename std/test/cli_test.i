(
	i/fail
	testlib = (test) = std/test
	std/cli
	std/list
	std/string
	(Some, None) = std/option
)

Options = (jobs = Int, verbose = Bool)

Pair = (Int, String)

Triple = (Int, String, Bool)

jobs = () -> cli.Parser(Int) {
	cli.option(
		cli.Names(long = "jobs", short = Some("j")),
		"Workers",
		cli.integer,
	)
}

verbose = () -> cli.Parser(Bool) {
	cli.flag(cli.Names(long = "verbose", short = Some("v")), "Show progress")
}

name = () -> cli.Parser(String) {
	cli.option(cli.Names(long = "name", short = Some("n")), "Name", cli.text)
}

options = () -> cli.Parser(Options) { cli.map2(jobs(), verbose(), Options) }

// A2/A3/A6/A7: the note's complete typed description and integer examples.
test("cli_options", () {
	assert(
		{
e4_call_arg_0 = options()
e4_call_arg_1 = ["--verbose", "--jobs=4"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(Options(jobs = 4, verbose = True)),
	)
	assert(
		{
e4_call_arg_0 = options()
e4_call_arg_1 = ["-j", "4"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(Options(jobs = 4, verbose = False)),
	)
	list.each([["--jobs=-2"], ["--jobs", "-2"], ["-j", "-2"]], (args) {
		assert(
			{
e4_call_arg_0 = jobs()
e4_call_arg_1 = args
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok(-2),
		)
	})
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs="]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(
				cli.InvalidArgument(
					index = 0,
					name = "--jobs",
					text = "",
					reason = "expected integer",
				),
			),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs", "--verbose"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(
				cli.InvalidArgument(
					index = 1,
					name = "--jobs",
					text = "--verbose",
					reason = "expected integer",
				),
			),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["-j", "bad"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(
				cli.InvalidArgument(
					index = 1,
					name = "--jobs",
					text = "bad",
					reason = "expected integer",
				),
			),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs=bad"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(
				cli.InvalidArgument(
					index = 0,
					name = "--jobs",
					text = "bad",
					reason = "expected integer",
				),
			),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Error(cli.MissingArgument(name = "--jobs")),
	)
	assert(
		{
e4_call_arg_0 = verbose()
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok(False),
	)
	Nil
})

// A1: validate spelling before duplicate names, then positional placement.
test("cli_definition_errors", () {
	stop_int = (_: String) -> Result(Int, String) {
		panic("reader called during validation")
	}
	stop_text = (_: String) -> Result(String, String) {
		panic("reader called during validation")
	}
	dup = cli.map2(
		cli.option(cli.Names(long = "jobs", short = None), "", stop_int),
		cli.option(cli.Names(long = "jobs", short = None), "", stop_text),
		Pair,
	)
	assert(
		fail.catch(() { cli.parse(dup, []) }) ==
			Error(cli.BadDefinition(cli.DuplicateName("--jobs"))),
	)
	assert(
		fail.catch(() { cli.parse(dup, ["--unknown"]) }) ==
			Error(cli.BadDefinition(cli.DuplicateName("--jobs"))),
	)
	assert(fail.catch(() { cli.help(dup, "app") }) == Error(cli.DuplicateName("--jobs")))
	p = jobs()
	assert(
		{
e4_call_arg_0 = cli.map2(p, p, (a, b) { a + b })
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.BadDefinition(cli.DuplicateName("--jobs"))),
	)
	assert(
		{
e4_call_arg_0 = cli.flag(cli.Names(long = "other", short = Some("ab")), "")
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.InvalidName("ab")),
	)
	assert(
		{
e4_call_arg_0 = cli.map2(jobs(), cli.flag(
				cli.Names(long = "other", short = Some("j")),
				"",
			), (a, _) { a })
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.DuplicateName("-j")),
	)
	assert(
		{
e4_call_arg_0 = cli.map2(jobs(), cli.flag(
				cli.Names(long = "jobs", short = Some("ab")),
				"",
			), (a, _) { a })
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.InvalidName("ab")),
	)
	list.each(["", "Jobs", "1jobs", "-jobs", "jo_bs", "jé", "a\u{0}", "a b"], (
		bad
	) {
		assert(
			{
e4_call_arg_0 = cli.flag(cli.Names(long = bad, short = Some("ab")), "")
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
				Error(cli.InvalidName(bad)),
		)
		assert(
			{
e4_call_arg_0 = cli.argument(bad, "", cli.text)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
				Error(cli.InvalidName(bad)),
		)
	})
	list.each(["", "1", "-", "é", "aa"], (bad) {
		assert(
			{
e4_call_arg_0 = cli.flag(cli.Names(long = "valid", short = Some(bad)), "")
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
				Error(cli.InvalidName(bad)),
		)
	})
	list.each(["a", "a0", "a-b", "a-", "a--b"], (good) {
		assert(
			{
e4_call_arg_0 = cli.flag(cli.Names(long = good, short = Some("Z")), "")
e4_call_arg_1 = ["-Z"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
				Ok(True),
		)
	})
	pos = cli.argument("input", "", cli.text)
	assert(
		{
e4_call_arg_0 = cli.map2(pos, pos, (a, _) { a })
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.DuplicateName("input")),
	)
	remaining = cli.rest("files", "", cli.text)
	assert(
		{
e4_call_arg_0 = cli.map2(remaining, pos, (a, _) { a })
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.MisplacedRest),
	)
	assert(
		{
e4_call_arg_0 = cli.map2(remaining, cli.rest("more", "", cli.text), list.append)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.MisplacedRest),
	)
	with_option = cli.map2(remaining, verbose(), (a, _) { a })
	assert(fail.catch(() { cli.parse(with_option, ["file", "-v"]) }) == Ok(["file"]))
	assert(
		{
e4_call_arg_0 = cli.map2(with_option, pos, (a, _) { a })
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.MisplacedRest),
	)
	// Option long/short names and positional names have independent namespaces.
	assert(
		{
e4_call_arg_0 = cli.map2(
				cli.option(
					cli.Names(long = "j", short = Some("j")),
					"",
					cli.integer,
				),
				cli.argument("j", "", cli.text),
				Pair,
			)
e4_call_arg_1 = ["-j", "3", "x"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(Pair(3, "x")),
	)
	Nil
})

// A2/A3: token spelling, separator ownership and values independent of readers.
test("cli_token_ownership", () {
	list.each([["--jobs"], ["--jobs", "--"], ["-j"], ["-j", "--"]], (args) {
		assert(
			{
e4_call_arg_0 = jobs()
e4_call_arg_1 = args
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
				Error(cli.MissingValue(index = 0, name = "--jobs")),
		)
	})
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs=1", "-j", "--"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.RepeatedOption(index = 1, name = "--jobs")),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["ignored", "-j", "--"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.MissingValue(index = 1, name = "--jobs")),
	)
	assert(
		{
e4_call_arg_0 = options()
e4_call_arg_1 = ["é", "--unknown"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.UnknownOption(index = 1, token = "--unknown")),
	)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name", "--verbose"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok("--verbose"),
	)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name="]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok(""),
	)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name=--"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok("--"),
	)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name=a=b"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok("a=b"),
	)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name", ""]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok(""),
	)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name", "é \u{0}"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok("é \u{0}"),
	)
	// A3 does not recognize the following --verbose while it belongs to name.
	assert(
		{
e4_call_arg_0 = cli.map2(name(), verbose(), (n, v) { (n, v) })
e4_call_arg_1 = ["--name", "--verbose"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(("--verbose", False)),
	)
	list.each([
		"--jo",
		"-vj",
		"-j3",
		"-j=3",
		"--no-verbose",
		"-2",
		"---",
		"--jobsx",
		"-é",
	], (raw) {
		assert(
			{
e4_call_arg_0 = options()
e4_call_arg_1 = [raw]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
				Error(cli.UnknownOption(index = 0, token = raw)),
		)
	})
	assert(
		{
e4_call_arg_0 = cli.rest("args", "", cli.text)
e4_call_arg_1 = ["--", "-2", "--", ""]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(["-2", "--", ""]),
	)
	assert(
		{
e4_call_arg_0 = cli.rest("args", "", cli.text)
e4_call_arg_1 = ["-", "a b", "é", "\u{0}"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(["-", "a b", "é", "\u{0}"]),
	)
	interspersed = cli.map2(cli.argument("file", "", cli.text), verbose(), (
		f,
		v
	) { (f, v) })
	assert(
		fail.catch(() { cli.parse(interspersed, ["input", "--verbose"]) }) == Ok(("input", True)),
	)
	assert(fail.catch(() { cli.parse(interspersed, ["-", "-v"]) }) == Ok(("-", True)))
	assert(
		{
e4_call_arg_0 = cli.argument("number", "", cli.integer)
e4_call_arg_1 = ["-2"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.UnknownOption(index = 0, token = "-2")),
	)
	assert(
		{
e4_call_arg_0 = cli.argument("number", "", cli.integer)
e4_call_arg_1 = ["--", "-2"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(-2),
	)
	Nil
})

// A4: repetition precedes arity checking and aliases identify the same leaf.
test("cli_repetition", () {
	assert(
		{
e4_call_arg_0 = verbose()
e4_call_arg_1 = ["--verbose=false"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.UnexpectedValue(index = 0, name = "--verbose")),
	)
	assert(
		{
e4_call_arg_0 = verbose()
e4_call_arg_1 = ["--verbose", "-v"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.RepeatedOption(index = 1, name = "--verbose")),
	)
	assert(
		{
e4_call_arg_0 = verbose()
e4_call_arg_1 = ["-v", "--verbose=false"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.RepeatedOption(index = 1, name = "--verbose")),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs=1", "--jobs"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.RepeatedOption(index = 1, name = "--jobs")),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["-j", "1", "--jobs", "--"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.RepeatedOption(index = 2, name = "--jobs")),
	)
	o = cli.optional(cli.Names(long = "name", short = Some("n")), "", cli.text)
	assert(
		fail.catch(() { cli.parse(o, ["--name=x", "-n", "y"]) }) ==
			Error(cli.RepeatedOption(index = 1, name = "--name")),
	)
	m = cli.many(cli.Names(long = "name", short = Some("n")), "", cli.text)
	assert(fail.catch(() { cli.parse(m, []) }) == Ok([]))
	assert(
		fail.catch(() { cli.parse(m, ["--name=a", "--name=", "--name=é"]) }) == Ok(["a", "", "é"]),
	)
	assert(
		fail.catch(() { cli.parse(m, ["-n", "a", "--name=", "-n", "é"]) }) == Ok(["a", "", "é"]),
	)
	Nil
})

// A5/A6: allocation is completed before any missing/conversion callback runs.
test("cli_error_order", () {
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--unknown"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.UnknownOption(index = 0, token = "--unknown")),
	)
	assert(
		{
e4_call_arg_0 = cli.argument("file", "", cli.text)
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.MissingArgument(name = "file")),
	)
	assert(
		{
e4_call_arg_0 = cli.pure(2)
e4_call_arg_1 = ["one", "two"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.ExtraArgument(index = 0, token = "one")),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs=bad", "extra"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.ExtraArgument(index = 1, token = "extra")),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["extra"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.ExtraArgument(index = 0, token = "extra")),
	)
	stop = (_: String) -> Result(Int, String) {
		panic("conversion before allocation")
	}
	assert(
		{
e4_call_arg_0 = cli.option(cli.Names(long = "jobs", short = None), "", stop)
e4_call_arg_1 = ["--jobs=x", "extra"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.ExtraArgument(index = 1, token = "extra")),
	)
	first = cli.option(cli.Names(long = "first", short = None), "", (
		_: String
	) -> Result(Int, String) { Error("first") })
	second = cli.option(cli.Names(long = "second", short = None), "", (
		_: String
	) -> Result(Int, String) { panic("later reader called") })
	p = cli.map2(first, second, (a, b) { a + b })
	second_error = cli.option(cli.Names(long = "second", short = None), "", (
		_: String
	) -> Result(Int, String) { Error("second") })
	assert(
		{
e4_call_arg_0 = cli.map2(first, second_error, (a, b) { a + b })
e4_call_arg_1 = ["--second=bad", "--first=bad"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(
				cli.InvalidArgument(
					index = 1,
					name = "--first",
					text = "bad",
					reason = "first",
				),
			),
	)
	assert(
		fail.catch(() { cli.parse(p, ["--second=bad", "--first=bad"]) }) ==
			Error(
				cli.InvalidArgument(
					index = 1,
					name = "--first",
					text = "bad",
					reason = "first",
				),
			),
	)
	assert(
		fail.catch(() { cli.parse(p, ["--second=bad"]) }) ==
			Error(cli.MissingArgument(name = "--first")),
	)
	assert(
		fail.catch(() { cli.parse(p, ["--first=bad", "--unknown"]) }) ==
			Error(cli.UnknownOption(index = 1, token = "--unknown")),
	)
	failing_many = cli.many(cli.Names(long = "num", short = None), "", (
		value: String
	) -> Result(Int, String) {
		value |>
			("1") { Ok(1) }
			| ("bad") { Error("stop") }
			| (_) { panic("later occurrence called") }
	})
	assert(
		fail.catch(() { cli.parse(failing_many, ["--num=1", "--num=bad", "--num=never"]) }) ==
			Error(
				cli.InvalidArgument(
					index = 1,
					name = "--num",
					text = "bad",
					reason = "stop",
				),
			),
	)
	assert(
		{
e4_call_arg_0 = cli.map(first, (_) -> Int { panic("mapped failed parser") })
e4_call_arg_1 = ["--first=x"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(
				cli.InvalidArgument(
					index = 0,
					name = "--first",
					text = "x",
					reason = "first",
				),
			),
	)
	Nil
})

// A5: optional absence and caller-supplied defaults; ordered positional rest.
test("cli_allocation", () {
	o = cli.optional(
		cli.Names(long = "jobs", short = Some("j")),
		"",
		cli.integer,
	)
	assert(fail.catch(() { cli.parse(o, []) }) == Ok(None))
	assert(fail.catch(() { cli.parse(o, ["-j", "2"]) }) == Ok(Some(2)))
	defaults = cli.map(o, (value) { value |> (None) { 4 } | (Some(n)) { n } })
	assert(fail.catch(() { cli.parse(defaults, []) }) == Ok(4))
	assert(fail.catch(() { cli.parse(defaults, ["--jobs=2"]) }) == Ok(2))
	p = cli.map2(cli.argument("first", "", cli.text), cli.rest(
		"tail",
		"",
		cli.text,
	), (a, b) { (a, b) })
	assert(fail.catch(() { cli.parse(p, ["a", "b", "c"]) }) == Ok(("a", ["b", "c"])))
	assert(fail.catch(() { cli.parse(p, ["a"]) }) == Ok(("a", [])))
	assert(fail.catch(() { cli.parse(p, []) }) == Error(cli.MissingArgument(name = "first")))
	assert(
		{
e4_call_arg_0 = cli.rest("tail", "", cli.text)
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok([]),
	)
	assert(
		{
e4_call_arg_0 = cli.map2(cli.argument("first", "", cli.text), cli.argument(
				"second",
				"",
				cli.text,
			), (a, b) { (a, b) })
e4_call_arg_1 = ["a", "b", "c"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.ExtraArgument(index = 2, token = "c")),
	)
	Nil
})

// A7: exactly std/int.parse's signed ASCII decimal grammar, with fixed error.
test("cli_converters", () {
	list.each([
		("0", 0),
		("+0", 0),
		("-0", 0),
		("-2", -2),
		("+12", 12),
		("001", 1),
		("9223372036854775808", 9223372036854775808),
	], (pair: (String, Int)) { assert(cli.integer(pair.0) == Ok(pair.1)) })
	list.each([
		"",
		"+",
		"-",
		" 1",
		"1 ",
		"1_0",
		"1.0",
		"１２",
		"١",
		"1\u{0}",
		"1\n",
	], (bad) { assert(cli.integer(bad) == Error("expected integer")) })
	list.each(["", "é", " a b ", "\u{0}"], (value) {
		assert(cli.text(value) == Ok(value))
	})
	Nil
})

// A6 and the observation laws: heterogeneous types and every bucket consumed.
test("cli_composition_laws", () {
	assert(
		{
e4_call_arg_0 = cli.map2(cli.pure(2), cli.pure("x"), Pair)
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(Pair(2, "x")),
	)
	assert(
		{
e4_call_arg_0 = cli.pure(1)
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok(1),
	)
	assert(
		{
e4_call_arg_0 = cli.pure("x")
e4_call_arg_1 = []
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok("x"),
	)
	p = options()
	identity = cli.map(p, (x) { x })
	list.each([
		[],
		["--jobs=4"],
		["--jobs=4", "-v"],
		["-v", "--unknown"],
		["extra"],
	], (args) {
		assert(fail.catch(() { cli.parse(identity, args) }) == fail.catch(() { cli.parse(p, args) }))
		assert(fail.catch(() { cli.parse(p, args) }) == fail.catch(() { cli.parse(p, args) }))
	})
	assert(fail.catch(() { cli.help(identity, "app") }) == fail.catch(() { cli.help(p, "app") }))
	// Reassociation includes map callbacks at the same left-to-right positions.
	left = cli.map2(
		cli.map2(jobs(), name(), Pair),
		verbose(),
		(p: Pair, v) {
			Pair(n, s) = p
			Triple(n, s, v)
		},
	)
	right = cli.map2(
		jobs(),
		cli.map2(name(), verbose(), (s, v) { (s, v) }),
		(n, pair: (String, Bool)) { Triple(n, pair.0, pair.1) },
	)
	list.each([
		["--jobs=4", "--name=x", "-v"],
		["--name=x", "--jobs=bad"],
		["--jobs=2"],
		["--unknown"],
	], (args) { assert(fail.catch(() { cli.parse(left, args) }) == fail.catch(() { cli.parse(right, args) })) })
	assert(fail.catch(() { cli.help(left, "app") }) == fail.catch(() { cli.help(right, "app") }))
	// Each leaf consumes one bucket; pure consumes none even within map2 trees.
	mixed = cli.map2(
		cli.map2(cli.pure("prefix"), jobs(), (s, n) { (s, n) }),
		cli.map2(cli.pure(False), verbose(), (_, v) { v }),
		(pair: (String, Int), v) { Triple(pair.1, pair.0, v) },
	)
	assert(
		fail.catch(() { cli.parse(mixed, ["--jobs=3", "-v"]) }) == Ok(Triple(3, "prefix", True)),
	)
	Nil
})

// A8: byte-exact help, all display forms, verbatim content, no callbacks.
test("cli_help", () {
	assert(
		{
e4_call_arg_0 = cli.optional(
				cli.Names(long = "jobs", short = Some("j")),
				"Workers",
				cli.integer,
			)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok("Usage: app\n  [--jobs, -j VALUE]\tWorkers\n"),
	)
	assert(
		{
e4_call_arg_0 = cli.pure(2)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} == Ok("Usage: app\n"),
	)
	assert(
		{
e4_call_arg_0 = verbose()
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok("Usage: app\n  [--verbose, -v]\tShow progress\n"),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok("Usage: app\n  --jobs, -j VALUE\tWorkers\n"),
	)
	assert(
		{
e4_call_arg_0 = cli.many(cli.Names(long = "name", short = None), "Names", cli.text)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok("Usage: app\n  [--name VALUE...]\tNames\n"),
	)
	assert(
		{
e4_call_arg_0 = cli.argument("file", "Input", cli.text)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok("Usage: app\n  <file>\tInput\n"),
	)
	assert(
		{
e4_call_arg_0 = cli.rest("files", "Inputs", cli.text)
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok("Usage: app\n  [<files>...]\tInputs\n"),
	)
	stop = (_: String) -> Result(Int, String) { panic("help called reader") }
	stopping = cli.map(cli.option(
		cli.Names(long = "jobs", short = None),
		"Workers\n\tverbatim",
		stop,
	), (_) -> String { panic("help called mapper") })
	assert(
		fail.catch(() { cli.help(stopping, "app\n--raw") }) ==
			Ok("Usage: app\n--raw\n  --jobs VALUE\tWorkers\n\tverbatim\n"),
	)
	stopping2 = cli.map2(stopping, cli.pure(False), (_, _) -> Int {
		panic("help called map2")
	})
	assert(
		fail.catch(() { cli.help(stopping2, "app") }) ==
			Ok("Usage: app\n  --jobs VALUE\tWorkers\n\tverbatim\n"),
	)
	assert(
		{
e4_call_arg_0 = options()
e4_call_arg_1 = "app"
fail.catch(() { cli.help(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(
				"Usage: app\n  --jobs, -j VALUE\tWorkers\n  [--verbose, -v]\tShow progress\n",
			),
	)
	// Help and version have no built-in parsing behavior, alone or mixed.
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--help"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.UnknownOption(index = 0, token = "--help")),
	)
	assert(
		{
e4_call_arg_0 = jobs()
e4_call_arg_1 = ["--jobs=1", "--help"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Error(cli.UnknownOption(index = 1, token = "--help")),
	)
	assert(
		{
e4_call_arg_0 = cli.flag(cli.Names(long = "help", short = None), "")
e4_call_arg_1 = ["--help"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(True),
	)
	assert(
		{
e4_call_arg_0 = cli.flag(cli.Names(long = "version", short = None), "")
e4_call_arg_1 = ["--version"]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(True),
	)
	Nil
})

// A2/A4 and §5.4: reverse accumulators preserve large inputs without suffix copies.
test("cli_long_inputs", () {
	value = string.repeat("é", 20000)
	assert(
		{
e4_call_arg_0 = name()
e4_call_arg_1 = ["--name=" <> value]
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} == Ok(value),
	)
	values = list.map(list.range(0, 1000), (n) {
		"--name=" <> { n % 2 |> (0) { "a" } | (_) { "é" } }
	})
	assert(
		{
e4_call_arg_0 = cli.many(cli.Names(long = "name", short = None), "", cli.text)
e4_call_arg_1 = values
fail.catch(() { cli.parse(e4_call_arg_0, e4_call_arg_1) })
} ==
			Ok(
				list.map(list.range(0, 1000), (n) {
					n % 2 |> (0) { "a" } | (_) { "é" }
				}),
			),
	)
	Nil
})
// §5.4: check the private decoder by compiling the production source as this
// test module, with an internal test appended. This verifies supplied empty
// buckets and the exact unused suffix. Restore this file even if a check fails.
// Run from the lane after building target/release/i:
// direnv exec /home/agent/Programming/i python3 -c 'from pathlib import Path; exec("\n".join(x[4:] for x in Path("std/test/cli_test.i").read_text().splitlines() if x.startswith("//| ")))'
//| import subprocess
//| from pathlib import Path
//| path = Path('std/test/cli_test.i')
//| original = path.read_text()
//| implementation = Path('std/src/cli.i').read_text()
//| case = '''
//| @test
//| cli_bucket_contract = () {
//|   p = map2(pure(2), map2(flag(Names("verbose", None), ""), optional(Names("jobs", None), "", integer), (v, n) { (v, n) }), (a, pair: (Bool, Option(Int))) { (a, pair.0, pair.1) })
//|   Parser(fields, decode) = p
//|   assert(list.length(fields) == 2)
//|   assert(decode([[], []]) == Ok(((2, False, None), [])))
//|   sentinel = [Token(9, "unused")]
//|   assert(decode([[Token(0, "--verbose")], [Token(1, "4")], sentinel]) == Ok(((2, True, Some(4)), [sentinel])))
//|   assert(parse(p, []) == Ok((2, False, None)))
//|   Nil
//| }
//| '''
//| try:
//|     path.write_text(implementation + '\n' + case)
//|     for level in ('0', '2'):
//|         result = subprocess.run(['target/release/i', 'test', '--project', 'std', '--opt', level, 'cli_bucket_contract'], capture_output=True, text=True)
//|         print('bucket contract opt ' + level, result.returncode, result.stdout, result.stderr, flush=True)
//|         assert result.returncode == 0 and '1 passed, 0 failed' in result.stdout
//| finally:
//|     path.write_text(original)
