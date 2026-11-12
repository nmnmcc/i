//// Pure command argument descriptions with typed results. Parser hides fixed
//// metadata and a decoder; Names, DefinitionError and CliError expose their
//// constructors. The pure parser has no reserved flag or implicit default.
////
//// pure stores a value; flag returns Bool; option/argument return a converted
//// value; optional returns Option; many/rest return List. map and map2 preserve
//// their result types. parse returns Result(A,CliError); help returns
//// Result(String,DefinitionError). text and integer are pure Result converters.
//// Conversion callbacks have type (String)->Result(A,String), without effects.
////
//// A1: validate all metadata first, in leaf order: ASCII long/positional names
//// [a-z][a-z0-9-]*, one ASCII letter for short names, spelling before duplicates
//// before positional order. Long, short and positional namespaces are separate.
//// Rest is unique and last among positional leaves; later options are allowed.
//// A2: tokens are unchanged strings with zero-based argument indexes. Recognize
//// exact --long, --long=value and -s until bare --. A lone - is positional;
//// all other leading-dash tokens are unknown. Options may follow positionals.
//// A3: flags take no value; other options consume an attached suffix, including
//// empty, or the next token unless absent or exactly --. Callback behavior does
//// not change token ownership. All recognized-option errors use --long names.
//// A4: singleton repetition wins over arity errors; many keeps occurrence order.
//// A5: allocate required positionals in order, then rest. Optional absence is
//// None; flags default False; many/rest default []; required leaves must occur.
//// A6: errors precede decoding: definition, scanning, extra positionals, then
//// missing/conversion errors in descriptor order. Conversion indexes identify
//// the value token, or option token for attached values; scanning indexes always
//// identify the option token. map/map2 run callbacks once after inner success.
//// A7: integer accepts an optional sign and ASCII decimal digits, otherwise
//// Error("expected integer"); text preserves all token values, including NUL.
//// A8: help validates metadata without callbacks. Usage plus program plus LF,
//// then two spaces, display name, tab, verbatim help and LF for each leaf.
//// Flags/optional/many are bracketed; value options add VALUE, many adds ...,
//// positionals use <name>, and rest uses [<name>...]. No automatic help action.
////
//// Laws: map(p,id) preserves parse and help observations. Regrouping map2 with
//// tuple reassociation preserves leaf and callback order. Repeated parse is
//// deterministic for deterministic, terminating callbacks. Closure equality is
//// not observational equality. Linear field lookup costs O(tokens*fields +
//// token codepoints), plus conversion; left-associated construction may cost
//// O(fields^2). Decoder stack depth follows builder depth. Text scans are linear.

////
//// Public type inventory:
//// Names { Names(long: String, short: Option(String)) }
//// DefinitionError { InvalidName(String) DuplicateName(String) MisplacedRest }
//// CliError { BadDefinition(DefinitionError) UnknownOption(index:Int,token:String)
////   MissingValue(index:Int,name:String) UnexpectedValue(index:Int,name:String)
////   RepeatedOption(index:Int,name:String) MissingArgument(name:String)
////   InvalidArgument(index:Int,name:String,text:String,reason:String)
////   ExtraArgument(index:Int,token:String) }
//// Parser(A) is opaque.
//// Pure signatures:
//// pure(A): (value: A) -> Parser(A)
//// flag: (names: Names, help: String) -> Parser(Bool)
//// option(A): (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(A)
//// optional(A): (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(Option(A))
//// many(A): (names: Names, help: String, read: (String) -> Result(A, String)) -> Parser(List(A))
//// argument(A): (name: String, help: String, read: (String) -> Result(A, String)) -> Parser(A)
//// rest(A): (name: String, help: String, read: (String) -> Result(A, String)) -> Parser(List(A))
//// map(A, B): (parser: Parser(A), with: (A) -> B) -> Parser(B)
//// map2(A, B, C): (first: Parser(A), second: Parser(B), with: (A, B) -> C) -> Parser(C)
//// parse(A): (parser: Parser(A), args: List(String)) -> Result(A, CliError)
//// help(A): (parser: Parser(A), program: String) -> Result(String, DefinitionError)
//// text: (text: String) -> Result(String, String)
//// integer: (text: String) -> Result(Int, String)
//// read obtains process arguments; run reports errors and runs a body.

(
	i/process
	i/stdio
	i/core
	i/fail
	std/list
	std/string
	std/int
	opt = (Option, Some, None) = std/option
)

(
	Parser
	Names(..)
	DefinitionError(..)
	CliError(..)
	pure
	flag
	option
	optional
	many
	argument
	rest
	map
	map2
	parse
	read
	run
	help
	text
	integer
)

Names {
	Names(long: String, short: Option(String))
}

DefinitionError {
	InvalidName(String)
	DuplicateName(String)
	MisplacedRest
}

CliError {
	BadDefinition(DefinitionError)
	UnknownOption(index: Int, token: String)
	MissingValue(index: Int, name: String)
	UnexpectedValue(index: Int, name: String)
	RepeatedOption(index: Int, name: String)
	MissingArgument(name: String)
	InvalidArgument(index: Int, name: String, text: String, reason: String)
	ExtraArgument(index: Int, token: String)
}

Token {
	Token(index: Int, text: String)
}

Kind {
	Flag
	Required
	Optional
	Repeated
	Positional
	Remaining
}

Field {
	Field(name: String, short: Option(String), help: String, kind: Kind)
}

Parser(A) {
	Parser(
		List(Field)
		(List(List(Token))) -> Result((A, List(List(Token))), CliError)
	)
}

pure(A) = (value: A) -> Parser(A) {
	Parser([], (buckets) { Ok((value, buckets)) })
}

flag = (names: Names, help: String) -> Parser(Bool) {
	Parser([Field(names.long, names.short, help, Flag)], (buckets) {
		buckets |> {
			[values, ..tail] -> Ok((values != [], tail))
			[]               -> panic("cli: missing flag bucket")
		}
	})
}

convert(A) = (
	token: Token
	name: String
	read: (String) -> Result(A, String)
) -> Result(A, CliError) {
	read(token.text) |> {
		Ok(value) -> Ok(value)
		Error(reason) -> Error(
			InvalidArgument(token.index, name, token.text, reason)
		)
	}
}

required(A) = (
	field: Field
	name: String
	read: (String) -> Result(A, String)
) -> Parser(A) {
	Parser([field], (buckets) {
		buckets |> {
			[]        -> panic("cli: missing required bucket")
			[[], .._] -> Error(MissingArgument(name))
			[[token, .._], ..tail] -> convert(token, name, read) |> {
				Error(error) -> Error(error)
				Ok(value)    -> Ok((value, tail))
			}
		}
	})
}

option(A) = (
	names: Names
	help: String
	read: (String) -> Result(A, String)
) -> Parser(A) {
	required(
		Field(names.long, names.short, help, Required)
		"--" <> names.long
		read
	)
}

argument(A) = (
	name: String
	help: String
	read: (String) -> Result(A, String)
) -> Parser(A) { required(Field(name, None, help, Positional), name, read) }

optional(A) = (
	names: Names
	help: String
	read: (String) -> Result(A, String)
) -> Parser(Option(A)) {
	Parser([Field(names.long, names.short, help, Optional)], (buckets) {
		buckets |> {
			[]           -> panic("cli: missing optional bucket")
			[[], ..tail] -> Ok((None, tail))
			[[token, .._], ..tail] -> convert(
				token
				"--" <> names.long
				read
			) |> {
				Error(error) -> Error(error)
				Ok(value)    -> Ok((Some(value), tail))
			}
		}
	})
}

convert_all(A) = (
	tokens: List(Token)
	name: String
	read: (String) -> Result(A, String)
) -> Result(List(A), CliError) {
	go = (remaining: List(Token), acc: List(A)) -> Result(List(A), CliError) {
		remaining |> {
			[] -> Ok(list.reverse(acc))
			[token, ..tail] -> convert(token, name, read) |> {
				Error(error) -> Error(error)
				Ok(value)    -> go(tail, [value, ..acc])
			}
		}
	}
	go(tokens, [])
}

repeated(A) = (
	field: Field
	name: String
	read: (String) -> Result(A, String)
) -> Parser(List(A)) {
	Parser([field], (buckets) {
		buckets |> {
			[] -> panic("cli: missing repeated bucket")
			[tokens, ..tail] -> convert_all(tokens, name, read) |> {
				Error(error) -> Error(error)
				Ok(values)   -> Ok((values, tail))
			}
		}
	})
}

many(A) = (
	names: Names
	help: String
	read: (String) -> Result(A, String)
) -> Parser(List(A)) {
	repeated(
		Field(names.long, names.short, help, Repeated)
		"--" <> names.long
		read
	)
}

rest(A) = (
	name: String
	help: String
	read: (String) -> Result(A, String)
) -> Parser(List(A)) {
	repeated(Field(name, None, help, Remaining), name, read)
}

map(A, B) = (parser: Parser(A), with: (A) -> B) -> Parser(B) {
	Parser(fields, decode) = parser
	Parser(fields, (buckets) {
		decode(buckets) |> {
			Error(error)      -> Error(error)
			Ok((value, tail)) -> Ok((with(value), tail))
		}
	})
}

map2(A, B, C) = (
	first: Parser(A)
	second: Parser(B)
	with: (A, B) -> C
) -> Parser(C) {
	Parser(left_fields, left) = first
	Parser(right_fields, right) = second
	Parser(list.append(left_fields, right_fields), (buckets) {
		left(buckets) |> {
			Error(error) -> Error(error)
			Ok((a, tail)) -> right(tail) |> {
				Error(error)       -> Error(error)
				Ok((b, remaining)) -> Ok((with(a, b), remaining))
			}
		}
	})
}

text = (text: String) -> Result(String, String) { Ok(text) }

integer = (text: String) -> Result(Int, String) {
	int.parse(text) |> {
		Ok(value) -> Ok(value)
		Error(_)  -> Error("expected integer")
	}
}

ascii_lower = (c: Int) -> Bool { c >= 97 && c <= 122 }

ascii_letter = (c: Int) -> Bool { ascii_lower(c) || { c >= 65 && c <= 90 } }

valid_name = (name: String) -> Bool {
	list.map(string.to_codepoints(name), string.utf_codepoint_to_int) |> {
		[] -> False
		[first, ..tail] -> ascii_lower(first) && list.all(tail, (c) {
			ascii_lower(c) || { c >= 48 && c <= 57 } || c == 45
		})
	}
}

valid_short = (name: String) -> Bool {
	list.map(string.to_codepoints(name), string.utf_codepoint_to_int) |> {
		[c] -> ascii_letter(c)
		_   -> False
	}
}

positional = (kind: Kind) -> Bool { kind == Positional || kind == Remaining }

validate = (fields: List(Field)) -> Result(Nil, DefinitionError) {
	go = (
		remaining: List(Field)
		longs: List(String)
		shorts: List(String)
		positions: List(String)
		had_rest: Bool
	) -> Result(Nil, DefinitionError) {
		remaining |> {
			[] -> Ok(Nil)
			[field, ..tail] -> {
				short_error = field.short |> {
					Some(name) -> { valid_short(name) } |> {
						True  -> None
						False -> Some(InvalidName(name))
					}
					None -> None
				}
				{ valid_name(field.name) } |> {
					False -> Error(InvalidName(field.name))
					True -> short_error |> {
						Some(error) -> Error(error)
						None -> { positional(field.kind) } |> {
							True -> {
								list.contains(positions, field.name)
							} |> {
								True -> Error(DuplicateName(field.name))
								False -> { had_rest } |> {
									True -> Error(MisplacedRest)
									False -> go(
										tail
										longs
										shorts
										[field.name, ..positions]
										field.kind == Remaining
									)
								}
							}
							False -> { list.contains(longs, field.name) } |> {
								True -> Error(DuplicateName("--" <> field.name))
								False -> field.short |> {
									Some(name) ? list.contains(
										shorts
										name
									) -> Error(DuplicateName("-" <> name))
									Some(name) -> go(
										tail
										[field.name, ..longs]
										[name, ..shorts]
										positions
										had_rest
									)
									None -> go(
										tail
										[field.name, ..longs]
										shorts
										positions
										had_rest
									)
								}
							}
						}
					}
				}
			}
		}
	}
	go(fields, [], [], [], False)
}

lookup = (
	fields: List(Field)
	name: String
	short: Bool
) -> Option((Int, Field)) {
	go = (remaining: List(Field), index: Int) -> Option((Int, Field)) {
		remaining |> {
			[] -> None
			[field, ..tail] -> {
				matches = { short } |> {
					True  -> field.short == Some(name)
					False -> field.name == name
				}
				{ positional(field.kind) == False && matches } |> {
					True  -> Some((index, field))
					False -> go(tail, index + 1)
				}
			}
		}
	}
	go(fields, 0)
}

bucket_at = (buckets: List(List(Token)), index: Int) -> List(Token) {
	buckets |> {
		[] -> panic("cli: bucket index out of range")
		[head, ..tail] -> { index == 0 } |> {
			True  -> head
			False -> bucket_at(tail, index - 1)
		}
	}
}

put = (
	buckets: List(List(Token))
	index: Int
	token: Token
) -> List(List(Token)) {
	buckets |> {
		[] -> panic("cli: bucket index out of range")
		[head, ..tail] -> { index == 0 } |> {
			True  -> [[token, ..head], ..tail]
			False -> [head, ..put(tail, index - 1, token)]
		}
	}
}

// The option name excludes leading dashes; only long options can attach values.
option_token = (token: String) -> Option((String, Bool, Option(String))) {
	string.to_codepoint_strings(token) |> {
		["-", "-", ..tail] -> {
			name = string.concat(tail)
			string.split_once(name, "=") |> {
				Some((before, value)) -> Some((before, False, Some(value)))
				None                  -> Some((name, False, None))
			}
		}
		["-", name] -> Some((name, True, None))
		_           -> None
	}
}

scan = (
	fields: List(Field)
	args: List(String)
) -> Result((List(List(Token)), List(Token)), CliError) {
	go = (
		remaining: List(String)
		index: Int
		options: Bool
		buckets: List(List(Token))
		positions: List(Token)
	) -> Result((List(List(Token)), List(Token)), CliError) {
		remaining |> {
			[] -> Ok((list.map(buckets, list.reverse), list.reverse(positions)))
			[raw, ..tail] -> {
				{ options && raw == "--" } |> {
					True -> go(tail, index + 1, False, buckets, positions)
					False -> {
						options && raw != "-" && string.starts_with(raw, "-")
					} |> {
						False -> go(
							tail
							index + 1
							options
							buckets
							[Token(index, raw), ..positions]
						)
						True -> option_token(raw) |> {
							None -> Error(UnknownOption(index, raw))
							Some((name, short, attached)) -> lookup(
								fields
								name
								short
							) |> {
								None -> Error(UnknownOption(index, raw))
								Some((slot, field)) -> {
									canonical = "--" <> field.name
									{
										field.kind != Repeated &&
											bucket_at(buckets, slot) != []
									} |> {
										True -> Error(
											RepeatedOption(index, canonical)
										)
										False -> field.kind |> {
											Flag -> attached |> {
												Some(_) -> Error(
													UnexpectedValue(
														index
														canonical
													)
												)
												None -> go(
													tail
													index + 1
													options
													put(
														buckets
														slot
														Token(index, raw)
													)
													positions
												)
											}
											_ -> attached |> {
												Some(value) -> go(
													tail
													index + 1
													options
													put(
														buckets
														slot
														Token(index, value)
													)
													positions
												)
												None -> tail |> {
													[] -> Error(
														MissingValue(
															index
															canonical
														)
													)
													["--", .._] -> Error(
														MissingValue(
															index
															canonical
														)
													)
													[value, ..rest] -> go(
														rest
														index + 2
														options
														put(
															buckets
															slot
															Token(
																index + 1
																value
															)
														)
														positions
													)
												}
											}
										}
									}
								}
							}
						}
					}
				}
			}
		}
	}
	go(args, 0, True, list.map(fields, (_) { [] }), [])
}

allocate = (
	fields: List(Field)
	buckets: List(List(Token))
	positions: List(Token)
) -> Result(List(List(Token)), CliError) {
	go = (
		remaining: List(Field)
		stored: List(List(Token))
		args: List(Token)
		acc: List(List(Token))
	) -> Result(List(List(Token)), CliError) {
		(remaining, stored) |> {
			([], []) -> args |> {
				[]           -> Ok(list.reverse(acc))
				[token, .._] -> Error(ExtraArgument(token.index, token.text))
			}
			([field, ..tail], [bucket, ..rest]) -> field.kind |> {
				Positional -> args |> {
					[]              -> go(tail, rest, [], [[], ..acc])
					[token, ..more] -> go(tail, rest, more, [[token], ..acc])
				}
				Remaining -> go(tail, rest, [], [args, ..acc])
				_         -> go(tail, rest, args, [bucket, ..acc])
			}
			_ -> panic("cli: field and bucket counts differ")
		}
	}
	go(fields, buckets, positions, [])
}

parse(A) = (parser: Parser(A), args: List(String)) -> Result(A, CliError) {
	Parser(fields, decode) = parser
	validate(fields) |> {
		Error(error) -> Error(BadDefinition(error))
		Ok(_) -> scan(fields, args) |> {
			Error(error) -> Error(error)
			Ok((buckets, positions)) -> allocate(
				fields
				buckets
				positions
			) |> {
				Error(error) -> Error(error)
				Ok(allocated) -> decode(allocated) |> {
					Error(error)    -> Error(error)
					Ok((value, [])) -> Ok(value)
					Ok(_)           -> panic("cli: decoder left unused buckets")
				}
			}
		}
	}
}

show_field = (field: Field) -> String {
	base = "--" <> field.name <> {
		field.short |> {
			Some(short) -> ", -" <> short
			None        -> ""
		}
	}
	field.kind |> {
		Flag       -> "[\(base)]"
		Required   -> base <> " VALUE"
		Optional   -> "[\(base) VALUE]"
		Repeated   -> "[\(base) VALUE...]"
		Positional -> "<\(field.name)>"
		Remaining  -> "[<\(field.name)>...]"
	}
}

help(A) = (
	parser: Parser(A)
	program: String
) -> Result(String, DefinitionError) {
	Parser(fields, _) = parser
	validate(fields) |> {
		Error(error) -> Error(error)
		Ok(_) -> Ok(
			string.concat([
				"Usage: "
				program
				"\n"
				..list.map(fields, (field: Field) {
					string.concat([
						"  "
						show_field(field)
						"\t"
						field.help
						"\n"
					])
				})
			])
		)
	}
}

/// Parses the arguments supplied by the process service.
read(A) = (parser: Parser(A)) { parse(parser, process.args()) }

/// Runs a command after parsing its arguments, reporting failures on stderr.
run(A, E, N) = (parser: Parser(A), program: String, body: (A) -> Nil ^ E ! N) {
	read(parser) |> {
		Ok(value) -> body(value)
		Error(error) -> {
			stdio.print_error(core.inspect(error))
			help(parser, program) |> {
				Ok(usage)         -> stdio.print_error(usage)
				Error(definition) -> stdio.print_error(core.inspect(definition))
			}
			fail.raise(error)
		}
	}
}
