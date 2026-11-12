(
	testlib = (test) = std/test
	i/core
	i/control
	i/fail
	std/csv
	std/list
	std/stream
	std/string
	std/dict
	std/schema
	(Some, None) = std/option
)

config = (delimiter: String, fields: Int, crlf: Bool) -> csv.Options {
	csv.options(delimiter, fields, crlf) |> {
		Ok(c)    -> c
		Error(_) -> panic("test options")
	}
}

PullProbe {
	PullProbe(note: () -> Nil)
}

note_pull = (probe ! PullProbe) -> Nil ! PullProbe { probe.note() }

test("parse_empty", () {
	assert(csv.parse("") == Ok([]))
	Nil
})

test("parse_one", () {
	assert(csv.parse("a") == Ok([["a"]]))
	Nil
})

test("parse_one_lf", () {
	assert(csv.parse("a\n") == Ok([["a"]]))
	Nil
})

test("parse_one_crlf", () {
	assert(csv.parse("a\r\n") == Ok([["a"]]))
	Nil
})

test("parse_row", () {
	assert(csv.parse("a,b,c") == Ok([["a", "b", "c"]]))
	Nil
})

test("parse_rows", () {
	assert(csv.parse("a,b\nc,d\n") == Ok([["a", "b"], ["c", "d"]]))
	Nil
})

test("parse_empty_fields", () {
	assert(csv.parse(",,") == Ok([["", "", ""]]))
	Nil
})

test("parse_trailing", () {
	assert(csv.parse("a,") == Ok([["a", ""]]))
	Nil
})

test("parse_leading", () {
	assert(csv.parse(",b") == Ok([["", "b"]]))
	Nil
})

test("parse_quoted_empty", () {
	assert(csv.parse("\"\"") == Ok([[""]]))
	Nil
})

test("parse_quoted", () {
	assert(csv.parse("\"a\",b") == Ok([["a", "b"]]))
	Nil
})

test("parse_quoted_delimiter", () {
	assert(csv.parse("\"a,b\",c") == Ok([["a,b", "c"]]))
	Nil
})

test("parse_escaped_quote", () {
	assert(csv.parse("\"a\"\"b\"") == Ok([["a\"b"]]))
	Nil
})

test("parse_just_quote", () {
	assert(csv.parse("\"\"\"\"") == Ok([["\""]]))
	Nil
})

test("parse_embedded_lf", () {
	assert(csv.parse("\"a\nb\",c") == Ok([["a\nb", "c"]]))
	Nil
})

test("parse_embedded_crlf", () {
	assert(csv.parse("\"a\r\nb\",c") == Ok([["a\nb", "c"]]))
	Nil
})

test("parse_bare_cr", () {
	assert(csv.parse("a\rb") == Ok([["a\rb"]]))
	Nil
})

test("parse_quoted_cr", () {
	assert(csv.parse("\"a\rb\"") == Ok([["a\rb"]]))
	Nil
})

test("parse_spaces", () {
	assert(csv.parse(" a , b ") == Ok([[" a ", " b "]]))
	Nil
})

test("parse_blank_lines", () {
	assert(csv.parse("\n\r\na\n\n") == Ok([["a"]]))
	Nil
})

test("parse_space_line", () {
	assert(csv.parse(" \n") == Ok([[" "]]))
	Nil
})

test("parse_unicode", () {
	assert(csv.parse("名字,值\n🦀,é") == Ok([["名字", "值"], ["🦀", "é"]]))
	Nil
})

test("parse_nul", () {
	assert(csv.parse("a\u{0000}b") == Ok([["a\u{0000}b"]]))
	Nil
})

test("parse_no_final_lf", () {
	assert(csv.parse("a,b\nc,d") == Ok([["a", "b"], ["c", "d"]]))
	Nil
})

test("parse_crlf_empty", () {
	assert(csv.parse("\r\n") == Ok([]))
	Nil
})

test("parse_quoted_delimiter_only", () {
	assert(csv.parse("\",\"") == Ok([[","]]))
	Nil
})

test("parse_quote_after_delimiter", () {
	assert(csv.parse("a,\"b\"") == Ok([["a", "b"]]))
	Nil
})

test("parse_embedded_blank", () {
	assert(csv.parse("\"\n\n\"") == Ok([["\n\n"]]))
	Nil
})

test("parse_last_empty", () {
	assert(csv.parse("\"\",") == Ok([["", ""]]))
	Nil
})

test("parse_bom", () {
	assert(csv.parse("﻿a,b") == Ok([["﻿a", "b"]]))
	Nil
})

test("error_bare_quote", () {
	assert(csv.parse("a\"b") == Error(csv.CsvError(line: 1, column: 2, reason: "bare quote")))
	Nil
})

test("error_after_quote", () {
	assert(
		csv.parse("\"a\"b") ==
			Error(csv.CsvError(line: 1, column: 4, reason: "data after closing quote"))
	)
	Nil
})

test("error_space_after_quote", () {
	assert(
		csv.parse("\"a\" ") ==
			Error(csv.CsvError(line: 1, column: 4, reason: "data after closing quote"))
	)
	Nil
})

test("error_missing_quote", () {
	assert(
		csv.parse("\"a") ==
			Error(csv.CsvError(line: 1, column: 3, reason: "unterminated quoted field"))
	)
	Nil
})

test("error_multiline_quote", () {
	assert(
		csv.parse("\"a\nb") ==
			Error(csv.CsvError(line: 2, column: 2, reason: "unterminated quoted field"))
	)
	Nil
})

test("error_unicode_position", () {
	assert(csv.parse("é\"") == Error(csv.CsvError(line: 1, column: 2, reason: "bare quote")))
	Nil
})

test("invalid_delimiter_0", () {
	csv.options("", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("invalid_delimiter_1", () {
	csv.options("ab", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("invalid_delimiter_2", () {
	csv.options("\n", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("invalid_delimiter_3", () {
	csv.options("\r", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("invalid_delimiter_4", () {
	csv.options("\"", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("invalid_delimiter_5", () {
	csv.options("\u{0000}", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("invalid_delimiter_6", () {
	csv.options("�", 0, False) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("invalid delimiter")
	}
	Nil
})

test("delimiter_semicolon", () {
	assert(
		csv.parse_with("a;b\nc;d", config(";", 0, False)) ==
			Ok([["a", "b"], ["c", "d"]])
	)
	Nil
})

test("delimiter_unicode", () {
	assert(
		csv.parse_with("a🦀b\nc🦀d", config("🦀", 0, False)) ==
			Ok([["a", "b"], ["c", "d"]])
	)
	Nil
})

test("delimiter_tab", () {
	assert(csv.parse_with("a\tb", config("\t", 0, False)) == Ok([["a", "b"]]))
	Nil
})

test("width_infer_error", () {
	csv.parse("a,b\nc") |> {
		Error(csv.CsvError(reason = reason, ..)) -> assert(
			reason == "wrong number of fields"
		)
		Ok(_) -> panic("width")
	}
	Nil
})

test("width_explicit", () {
	assert(csv.parse_with("a,b", config(",", 2, False)) == Ok([["a", "b"]]))
	Nil
})

test("width_explicit_error", () {
	csv.parse_with("a,b", config(",", 3, False)) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("width")
	}
	Nil
})

test("width_variable", () {
	assert(
		csv.parse_with("a\nb,c", config(",", -1, False)) ==
			Ok([["a"], ["b", "c"]])
	)
	Nil
})

test("print_plain", () {
	assert(csv.print([["a", "b"]]) == "a,b\n")
	Nil
})

test("print_quote", () {
	assert(csv.print([["a\"b"]]) == "\"a\"\"b\"\n")
	Nil
})

test("print_delimiter", () {
	assert(csv.print([["a,b"]]) == "\"a,b\"\n")
	Nil
})

test("print_lf", () {
	assert(csv.print([["a\nb"]]) == "\"a\nb\"\n")
	Nil
})

test("print_cr", () {
	assert(csv.print([["a\rb"]]) == "\"a\rb\"\n")
	Nil
})

test("print_spaces", () {
	assert(csv.print([[" a "]]) == " a \n")
	Nil
})

test("print_single_empty", () {
	assert(csv.print([[""]]) == "\"\"\n")
	Nil
})

test("print_empty_fields", () {
	assert(csv.print([["", ""]]) == ",\n")
	Nil
})

test("print_empty_rows", () {
	assert(csv.print([[]]) == "\"\"\n")
	Nil
})

test("print_none", () {
	assert(csv.print([]) == "")
	Nil
})

test("print_unicode", () {
	assert(csv.print([["é", "🦀"]]) == "é,🦀\n")
	Nil
})

test("print_crlf", () {
	assert(
		csv.print_with([["a", "b"], ["c", "d"]], config(",", 0, True)) ==
			"a,b\r\nc,d\r\n"
	)
	Nil
})

test("print_custom_delimiter", () {
	assert(
		csv.print_with([["a;b", "c"]], config(";", 0, False)) == "\"a;b\";c\n"
	)
	Nil
})

test("stream_boundaries_0", () {
	cases = [["", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_1", () {
	cases = [["", "a"], ["a", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_2", () {
	cases = [["", "a\n"], ["a", "\n"], ["a\n", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_3", () {
	cases = [["", "a\r\n"], ["a", "\r\n"], ["a\r", "\n"], ["a\r\n", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_4", () {
	cases = [
		["", "a,b,c"]
		["a", ",b,c"]
		["a,", "b,c"]
		["a,b", ",c"]
		["a,b,", "c"]
		["a,b,c", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a", "b", "c"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_5", () {
	cases = [
		["", "a,b\nc,d\n"]
		["a", ",b\nc,d\n"]
		["a,", "b\nc,d\n"]
		["a,b", "\nc,d\n"]
		["a,b\n", "c,d\n"]
		["a,b\nc", ",d\n"]
		["a,b\nc,", "d\n"]
		["a,b\nc,d", "\n"]
		["a,b\nc,d\n", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a", "b"]), Ok(["c", "d"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_6", () {
	cases = [["", ",,"], [",", ","], [",,", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["", "", ""])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_7", () {
	cases = [["", "a,"], ["a", ","], ["a,", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a", ""])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_8", () {
	cases = [["", ",b"], [",", "b"], [",b", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["", "b"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_9", () {
	cases = [["", "\"\""], ["\"", "\""], ["\"\"", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok([""])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_10", () {
	cases = [
		["", "\"a\",b"]
		["\"", "a\",b"]
		["\"a", "\",b"]
		["\"a\"", ",b"]
		["\"a\",", "b"]
		["\"a\",b", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a", "b"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_11", () {
	cases = [
		["", "\"a,b\",c"]
		["\"", "a,b\",c"]
		["\"a", ",b\",c"]
		["\"a,", "b\",c"]
		["\"a,b", "\",c"]
		["\"a,b\"", ",c"]
		["\"a,b\",", "c"]
		["\"a,b\",c", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a,b", "c"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_12", () {
	cases = [
		["", "\"a\"\"b\""]
		["\"", "a\"\"b\""]
		["\"a", "\"\"b\""]
		["\"a\"", "\"b\""]
		["\"a\"\"", "b\""]
		["\"a\"\"b", "\""]
		["\"a\"\"b\"", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a\"b"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_13", () {
	cases = [
		["", "\"\"\"\""]
		["\"", "\"\"\""]
		["\"\"", "\"\""]
		["\"\"\"", "\""]
		["\"\"\"\"", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["\""])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_14", () {
	cases = [
		["", "\"a\nb\",c"]
		["\"", "a\nb\",c"]
		["\"a", "\nb\",c"]
		["\"a\n", "b\",c"]
		["\"a\nb", "\",c"]
		["\"a\nb\"", ",c"]
		["\"a\nb\",", "c"]
		["\"a\nb\",c", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a\nb", "c"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_15", () {
	cases = [
		["", "\"a\r\nb\",c"]
		["\"", "a\r\nb\",c"]
		["\"a", "\r\nb\",c"]
		["\"a\r", "\nb\",c"]
		["\"a\r\n", "b\",c"]
		["\"a\r\nb", "\",c"]
		["\"a\r\nb\"", ",c"]
		["\"a\r\nb\",", "c"]
		["\"a\r\nb\",c", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a\nb", "c"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_16", () {
	cases = [["", "a\rb"], ["a", "\rb"], ["a\r", "b"], ["a\rb", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a\rb"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_17", () {
	cases = [
		["", "\"a\rb\""]
		["\"", "a\rb\""]
		["\"a", "\rb\""]
		["\"a\r", "b\""]
		["\"a\rb", "\""]
		["\"a\rb\"", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a\rb"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_18", () {
	cases = [
		["", " a , b "]
		[" ", "a , b "]
		[" a", " , b "]
		[" a ", ", b "]
		[" a ,", " b "]
		[" a , ", "b "]
		[" a , b", " "]
		[" a , b ", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok([" a ", " b "])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_19", () {
	cases = [
		["", "\n\r\na\n\n"]
		["\n", "\r\na\n\n"]
		["\n\r", "\na\n\n"]
		["\n\r\n", "a\n\n"]
		["\n\r\na", "\n\n"]
		["\n\r\na\n", "\n"]
		["\n\r\na\n\n", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_20", () {
	cases = [["", " \n"], [" ", "\n"], [" \n", ""]]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok([" "])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_21", () {
	cases = [
		["", "名字,值\n🦀,é"]
		["名", "字,值\n🦀,é"]
		["名字", ",值\n🦀,é"]
		["名字,", "值\n🦀,é"]
		["名字,值", "\n🦀,é"]
		["名字,值\n", "🦀,é"]
		["名字,值\n🦀", ",é"]
		["名字,值\n🦀,", "é"]
		["名字,值\n🦀,é", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["名字", "值"]), Ok(["🦀", "é"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_22", () {
	cases = [
		["", "a\u{0000}b"]
		["a", "\u{0000}b"]
		["a\u{0000}", "b"]
		["a\u{0000}b", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a\u{0000}b"])]
		)
		Nil
	})
	Nil
})

test("stream_boundaries_23", () {
	cases = [
		["", "a,b\nc,d"]
		["a", ",b\nc,d"]
		["a,", "b\nc,d"]
		["a,b", "\nc,d"]
		["a,b\n", "c,d"]
		["a,b\nc", ",d"]
		["a,b\nc,", "d"]
		["a,b\nc,d", ""]
	]
	list.each(cases, (chunks) {
		assert(
			stream.to_list(
				csv.rows(stream.from_list(chunks), csv.defaults())
			) ==
				[Ok(["a", "b"]), Ok(["c", "d"])]
		)
		Nil
	})
	Nil
})

test("stream_scalar_chunks", () {
	chunks = string.to_codepoint_strings("名字,\"🦀\r\né\"\r\n")
	assert(
		stream.to_list(csv.rows(stream.from_list(chunks), csv.defaults())) ==
			[Ok(["名字", "🦀\né"])]
	)
	Nil
})

test("stream_stops_on_error", () {
	assert(
		stream.to_list(
			csv.rows(stream.from_list(["a\"\nb\n"]), csv.defaults())
		) ==
			[Error(csv.CsvError(line: 1, column: 2, reason: "bare quote"))]
	)
	Nil
})

test("stream_early_termination", () {
	tail = stream.make(() -> stream.Step(String, Never, Never) {
		panic("tail must not be pulled")
	})
	source = stream.make(() { stream.Next("a,b\n", tail) })
	assert(
		stream.to_list(stream.take(csv.rows(source, csv.defaults()), 1)) ==
			[Ok(["a", "b"])]
	)
	Nil
})

test("stream_source_error", () {
	source = stream.make(() -> stream.Step(String, String, Never) ^ String {
		fail.raise("source error")
	})
	assert(
		fail.catch(() { stream.to_list(csv.rows(source, csv.defaults())) }) ==
			Error("source error")
	)
	Nil
})

test("stream_replay", () {
	rows = csv.rows(stream.from_list(["a,b\nc,d"]), csv.defaults())
	assert(stream.to_list(rows) == stream.to_list(rows))
	Nil
})

test("header", () {
	result = csv.parse_header("name,age\ni,1\n", csv.defaults())
	assert(result == Ok([dict.from_list([("name", "i"), ("age", "1")])]))
	Nil
})

test("header_empty", () {
	assert(csv.parse_header("", csv.defaults()) == Ok([]))
	Nil
})

test("header_only", () {
	assert(csv.parse_header("a,b", csv.defaults()) == Ok([]))
	Nil
})

test("header_duplicate", () {
	assert(
		csv.parse_header("a,a\n1,2", csv.defaults()) ==
			Error(csv.CsvError(line: 1, column: 1, reason: "duplicate header"))
	)
	Nil
})

test("header_width", () {
	csv.parse_header("a,b\n1", config(",", -1, False)) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("header width")
	}
	Nil
})

test("header_empty_name", () {
	assert(
		csv.parse_header(",b\n1,2", csv.defaults()) ==
			Ok([dict.from_list([("", "1"), ("b", "2")])])
	)
	Nil
})

test("stream_records", () {
	assert(
		stream.to_list(
			csv.records(stream.from_list(["a,b\n", "1,2\n"]), csv.defaults())
		) ==
			[Ok(dict.from_list([("a", "1"), ("b", "2")]))]
	)
	Nil
})

test("schema", () {
	codec = schema.dict(schema.string)
	row = dict.from_list([("a", "one"), ("b", "two")])
	assert(csv.decode(codec, row) == Ok(row))
	Nil
})

test("schema_error", () {
	codec = schema.dict(schema.int)
	csv.decode(codec, dict.from_list([("a", "one")])) |> {
		Error(_) -> Nil
		Ok(_)    -> panic("string must not coerce")
	}
	Nil
})

test("large_roundtrip", () {
	rows = list.repeat(["hello", "quoted,field", "é🦀"], 2000)
	assert(csv.parse(csv.print(rows)) == Ok(rows))
	Nil
})

test("width_error_record_line", () {
	assert(
		csv.parse("a,b\n\nc\n") ==
			Error(csv.CsvError(line: 3, column: 1, reason: "wrong number of fields"))
	)
	Nil
})

test("header_width_error_line", () {
	assert(
		csv.parse_header("a,b\n\n1\n", config(",", -1, False)) ==
			Error(csv.CsvError(line: 3, column: 1, reason: "header width differs from row"))
	)
	Nil
})

test("duplicate_header_error_line", () {
	assert(
		csv.parse_header("\n\na,a\n", csv.defaults()) ==
			Error(csv.CsvError(line: 3, column: 1, reason: "duplicate header"))
	)
	Nil
})

test("bare_cr_lookahead_pulls_once", () -> Nil ! Assert {
	events = control.boundary((boundary) {
		! PullProbe = PullProbe(note: () {
			control.suspend(boundary, (continuation) {
				[1, ..control.resume(continuation, Nil)]
			})
		})
		last = stream.make(
			() -> stream.Step(String, Never, PullProbe) ! PullProbe {
				note_pull()
				stream.Next("b\n", stream.new())
			}
		)
		source = stream.append(stream.from_list(["a\r"]), last)
		assert(
			stream.to_list(csv.rows(source, csv.defaults())) == [Ok(["a\rb"])]
		)
		[]
	})
	assert(events == [1])
	Nil
})

test("bare_cr_eof_pulls_once", () -> Nil ! Assert {
	events = control.boundary((boundary) {
		! PullProbe = PullProbe(note: () {
			control.suspend(boundary, (continuation) {
				[1, ..control.resume(continuation, Nil)]
			})
		})
		last = stream.make(
			() -> stream.Step(String, Never, PullProbe) ! PullProbe {
				note_pull()
				stream.Done
			}
		)
		source = stream.append(stream.from_list(["a\r"]), last)
		assert(
			stream.to_list(csv.rows(source, csv.defaults())) == [Ok(["a\r"])]
		)
		[]
	})
	assert(events == [1])
	Nil
})
