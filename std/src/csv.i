//// Strict CSV rows, following RFC 4180 and Go encoding/csv.
//// CRLF is normalized to LF, including inside quoted fields. Blank lines
//// are skipped; spaces are data. Errors use one-based scalar line/column.
//// Streams consume chunks only on demand, retain at most one row plus the
//// current chunk, and end after the first error. Pulls are replayable.
(
	i/core
	i/fail
	std/list
	std/string
	std/dict
	std/json
	std/schema
	std/stream
	(Option, Some, None) = std/option
)

(
	Options
	CsvError(..)
	options
	defaults
	parse
	parse_with
	parse_header
	print
	print_with
	rows
	records
	decode
)

Options {
	Options(delimiter: Int, fields: Int, crlf: Bool)
}

CsvError {
	CsvError(line: Int, column: Int, reason: String)
}

/// Comma delimiter, infer row width, LF output.
defaults = () -> Options { Options(44, 0, False) }

/// A single scalar delimiter, expected width (0 infer, negative variable),
/// and output line ending. Quotes, NUL, CR, LF, U+FFFD are invalid delimiters.
/// `csv.options(delimiter: ";", fields: -1, crlf: True)`.
options = (
	delimiter: String
	fields: Int
	crlf: Bool
) -> Result(Options, CsvError) {
	core.codepoints(delimiter) |> {
		[c] -> {
			n = core.codepoint_to_int(c)
			{ n == 0 || n == 34 || n == 10 || n == 13 || n == 65533 } |> {
				True  -> Error(CsvError(1, 1, "invalid delimiter"))
				False -> Ok(Options(n, fields, crlf))
			}
		}
		_ -> Error(CsvError(1, 1, "delimiter must be one scalar"))
	}
}

Cursor(E, N) {
	Cursor(List(Int), stream.Stream(String, E, N), Int, Int)
}

raw(E, N) = (cursor: Cursor(E, N)) -> (Option(Int), Cursor(E, N)) ^ E ! N {
	Cursor(pending, source, line, column) = cursor
	pending |> {
		[c, ..rest] -> {
			(l, k) = { c == 10 } |> {
				True  -> (line + 1, 1)
				False -> (line, column + 1)
			}
			(Some(c), Cursor(rest, source, l, k))
		}
		[] -> stream.pull(source) |> {
			stream.Done -> (None, cursor)
			stream.Next(chunk, tail) -> raw(
				Cursor(
					list.map(core.codepoints(chunk), core.codepoint_to_int)
					tail
					line
					column
				)
			)
		}
	}
}

next(E, N) = (cursor: Cursor(E, N)) -> (Option(Int), Cursor(E, N)) ^ E ! N {
	(c, tail) = raw(cursor)
	c |> {
		Some(13) -> {
			(d, after) = raw(tail)
			d |> {
				Some(10) -> (Some(10), after)
				_        -> (c, tail)
			}
		}
		_ -> (c, tail)
	}
}

text = (reversed: List(Int)) -> String {
	core.concat(list.map(list.reverse(reversed), (c) {
		<<c:u(32)>> |> {
			<<p:utf32be_codepoint>> -> core.from_codepoints([p])
			_                       -> panic("csv: scalar invariant")
		}
	}))
}

ReadRow {
	ReadRow(List(String), Int)
}

cursor_line(E, N) = (cursor: Cursor(E, N)) -> Int {
	Cursor(_, _, line, _) = cursor
	line
}

// Modes: 0 field start, 1 bare field, 2 quoted field, 3 closing quote.
read_row(E, N) = (
	cursor: Cursor(E, N)
	config: Options
	mode: Int
	field: List(Int)
	fields: List(String)
	started: Bool
	startline: Int
) -> Result((Option(ReadRow), Cursor(E, N)), CsvError) ^ E ! N {
	Cursor(_, _, line, column) = cursor
	(c, tail) = next(cursor)
	c |> {
		None -> mode |> {
			2 -> Error(CsvError(line, column, "unterminated quoted field"))
			_ -> started |> {
				False -> Ok((None, tail))
				True -> Ok((
					Some(
						ReadRow(
							list.reverse([text(field), ..fields])
							startline
						)
					)
					tail
				))
			}
		}
		Some(ch) -> {
			{ mode == 2 } |> {
				True -> ch |> {
					34 -> read_row(
						tail
						config
						3
						field
						fields
						True
						startline
					)
					_ -> read_row(
						tail
						config
						2
						[ch, ..field]
						fields
						True
						startline
					)
				}
				False -> { ch == config.delimiter } |> {
					True -> read_row(
						tail
						config
						0
						[]
						[text(field), ..fields]
						True
						startline
					)
					False -> ch |> {
						10 -> started |> {
							False -> read_row(
								tail
								config
								0
								[]
								[]
								False
								cursor_line(tail)
							)
							True -> Ok((
								Some(
									ReadRow(
										list.reverse([text(field), ..fields])
										startline
									)
								)
								tail
							))
						}
						34 -> mode |> {
							0 -> read_row(
								tail
								config
								2
								field
								fields
								True
								startline
							)
							3 -> read_row(
								tail
								config
								2
								[34, ..field]
								fields
								True
								startline
							)
							_ -> Error(CsvError(line, column, "bare quote"))
						}
						_ -> mode |> {
							3 -> Error(
								CsvError(
									line
									column
									"data after closing quote"
								)
							)
							_ -> read_row(
								tail
								config
								1
								[ch, ..field]
								fields
								True
								startline
							)
						}
					}
				}
			}
		}
	}
}

row_stream(E, N) = (
	cursor: Cursor(E, N)
	config: Options
	width: Int
) -> stream.Stream(Result(ReadRow, CsvError), E, N) {
	stream.make(() {
		parsed = read_row(cursor, config, 0, [], [], False, cursor_line(cursor))
		parsed |> {
			Error(e)      -> stream.Next(Error(e), stream.new())
			Ok((None, _)) -> stream.Done
			Ok((Some(ReadRow(row, startline)), tail)) -> {
				n = list.length(row)
				expected = { width == 0 } |> {
					True  -> n
					False -> width
				}
				{ expected > 0 && n != expected } |> {
					True -> stream.Next(
						Error(CsvError(startline, 1, "wrong number of fields"))
						stream.new()
					)
					False -> stream.Next(
						Ok(ReadRow(row, startline))
						row_stream(tail, config, expected)
					)
				}
			}
		}
	})
}

/// Parses a stream of arbitrary text chunks, emitting one row per pull.
/// `csv.rows(stream.from_list(["a,", "b\n"]), csv.defaults())`.
rows(E, N) = (
	source: stream.Stream(String, E, N)
	config: Options
) -> stream.Stream(Result(List(String), CsvError), E, N) {
	stream.map(
		row_stream(Cursor([], source, 1, 1), config, config.fields)
		(result) {
			result |> {
				Error(e)            -> Error(e)
				Ok(ReadRow(row, _)) -> Ok(row)
			}
		}
	)
}

/// Parses comma-separated text, inferring the width from the first row.
/// `csv.parse("a,b\n1,2\n")` gives `Ok([["a", "b"], ["1", "2"]])`.
parse = (input: String) -> Result(List(List(String)), CsvError) {
	parse_with(input, defaults())
}

/// Parses with an explicit delimiter and row-width policy.
parse_with = (
	input: String
	config: Options
) -> Result(List(List(String)), CsvError) {
	fail.catch(() {
		stream.fold(rows(stream.from_list([input]), config), [], (acc, row) {
			[fail.from_result(row), ..acc]
		})
			|> list.reverse
	})
}

header = (names: List(String), line: Int) -> Result(List(String), CsvError) {
	{ list.length(list.unique(names)) == list.length(names) } |> {
		True  -> Ok(names)
		False -> Error(CsvError(line, 1, "duplicate header"))
	}
}

named = (
	names: List(String)
	row: List(String)
	line: Int
) -> Result(dict.Dict(String, String), CsvError) {
	{ list.length(names) != list.length(row) } |> {
		True  -> Error(CsvError(line, 1, "header width differs from row"))
		False -> Ok(dict.from_list(list.zip(names, row)))
	}
}

record_stream(E, N) = (
	source: stream.Stream(Result(ReadRow, CsvError), E, N)
	names: List(String)
) -> stream.Stream(Result(dict.Dict(String, String), CsvError), E, N) {
	stream.make(() {
		stream.pull(source) |> {
			stream.Done              -> stream.Done
			stream.Next(Error(e), _) -> stream.Next(Error(e), stream.new())
			stream.Next(Ok(ReadRow(row, line)), tail) -> named(
				names
				row
				line
			) |> {
				Error(e)  -> stream.Next(Error(e), stream.new())
				Ok(value) -> stream.Next(Ok(value), record_stream(tail, names))
			}
		}
	})
}

/// Uses the first row as unique column names. Empty input yields no records.
records(E, N) = (
	source: stream.Stream(String, E, N)
	config: Options
) -> stream.Stream(Result(dict.Dict(String, String), CsvError), E, N) {
	stream.make(() {
		stream.pull(
			row_stream(Cursor([], source, 1, 1), config, config.fields)
		) |> {
			stream.Done              -> stream.Done
			stream.Next(Error(e), _) -> stream.Next(Error(e), stream.new())
			stream.Next(Ok(ReadRow(names, line)), tail) -> header(
				names
				line
			) |> {
				Error(e) -> stream.Next(Error(e), stream.new())
				Ok(keys) -> stream.pull(record_stream(tail, keys))
			}
		}
	})
}

/// Header-mode convenience parser: `csv.parse_header("name\ni\n", csv.defaults())`.
parse_header = (
	input: String
	config: Options
) -> Result(List(dict.Dict(String, String)), CsvError) {
	fail.catch(() {
		stream.fold(records(stream.from_list([input]), config), [], (acc, row) {
			[fail.from_result(row), ..acc]
		})
			|> list.reverse
	})
}

/// Decodes a named row as a JSON object of strings using an ordinary schema.
/// Compose `schema.string` with `schema.map` for domain conversions.
decode(A, E) = (
	codec: schema.Schema(A, E)
	row: dict.Dict(String, String)
) -> Result(A, E) {
	schema.decode(codec, json.Object(list.map(dict.to_list(row), (entry) {
		(k, v) = entry
		(k, json.Text(v))
	})))
}

quote = (field: String, delimiter: String, single_empty: Bool) -> String {
	{
		single_empty ||
			string.contains(field, delimiter) ||
			string.contains(field, "\"") ||
			string.contains(field, "\r") ||
			string.contains(field, "\n")
	} |> {
		True  -> "\"" <> string.replace(field, "\"", "\"\"") <> "\""
		False -> field
	}
}

/// Prints minimal quoting, with one LF after every row.
/// `csv.print([["a", "b,c"]])` gives `"a,\"b,c\"\n"`.
print = (rows: List(List(String))) -> String { print_with(rows, defaults()) }

/// Prints with the configured delimiter and LF/CRLF. Empty rows print as `""`.
print_with = (rows: List(List(String)), config: Options) -> String {
	delimiter = text([config.delimiter])
	ending = config.crlf |> {
		True  -> "\r\n"
		False -> "\n"
	}
	core.concat(list.map(rows, (row) {
		values = row |> {
			[] -> [""]
			_  -> row
		}
		string.join(
			list.map(values, (field) {
				quote(field, delimiter, values == [""])
			})
			delimiter
		) <>
			ending
	}))
}
