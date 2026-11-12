//// JSON values and RFC 8259 text encoding. Offsets are UTF-8 byte offsets.
(
	i/core
	i/fail
	std/list
	std/string
	std/bit_array
	std/int
	std/float
	(Option, Some, None) = std/option
)

(
	Json
	Null(..)
	Boolean(..)
	Integer(..)
	Number(..)
	Text(..)
	Array(..)
	Object(..)
	JsonError(..)
	parse
	print
	pretty
	field
	index
	as_bool
	as_int
	as_float
	as_string
	as_array
	as_object
)

Json = Null | Boolean | Integer | Number | Text | Array | Object

Null {
	Null
}

Boolean {
	Boolean(Bool)
}

Integer {
	Integer(Int)
}

Number {
	Number(Float)
}

Text {
	Text(String)
}

Array {
	Array(List(Json))
}

Object {
	Object(List((String, Json)))
}

JsonError {
	JsonError(offset: Int, reason: String)
}

parse = (text: String) -> Result(Json, JsonError) {
	fail.catch(() {
		xs = octets(<<text:utf8>>)
		(value, rest, at) = value(xs, 0)
		(tail, end) = whitespace(rest, at)
		tail |> {
			[] -> value
			_  -> fail.raise(JsonError(end, "trailing data"))
		}
	})
}

whitespace = (xs: List(Int), at: Int) -> (List(Int), Int) {
	xs |> {
		[x, ..tail] ? x == 32 || x == 9 || x == 10 || x == 13 -> whitespace(
			tail
			at + 1
		)
		_ -> (xs, at)
	}
}

value = (input: List(Int), offset: Int) -> (Json, List(Int), Int) ^ JsonError {
	(xs, at) = whitespace(input, offset)
	xs |> {
		[110, 117, 108, 108, ..rest]     -> (Null, rest, at + 4)
		[116, 114, 117, 101, ..rest]     -> (Boolean(True), rest, at + 4)
		[102, 97, 108, 115, 101, ..rest] -> (Boolean(False), rest, at + 5)
		[34, ..rest] -> {
			(s, tail, end) = read_string(rest, at + 1, [])
			(Text(s), tail, end)
		}
		[91, ..rest]                   -> array_start(rest, at + 1)
		[123, ..rest]                  -> object_start(rest, at + 1)
		[x, .._] ? x == 45 || digit(x) -> number(xs, at)
		_                              -> fail.raise(JsonError(at, "expected value"))
	}
}

array_start = (xs: List(Int), at: Int) -> (Json, List(Int), Int) ^ JsonError {
	(rest, pos) = whitespace(xs, at)
	rest |> {
		[93, ..tail] -> (Array([]), tail, pos + 1)
		_            -> array_items(rest, pos, [])
	}
}

array_items = (
	xs: List(Int)
	at: Int
	acc: List(Json)
) -> (Json, List(Int), Int) ^ JsonError {
	(v, rest, pos) = value(xs, at)
	(tail, end) = whitespace(rest, pos)
	tail |> {
		[93, ..after] -> (Array(list.reverse([v, ..acc])), after, end + 1)
		[44, ..after] -> array_items(after, end + 1, [v, ..acc])
		_             -> fail.raise(JsonError(end, "expected comma or closing bracket"))
	}
}

object_start = (xs: List(Int), at: Int) -> (Json, List(Int), Int) ^ JsonError {
	(rest, pos) = whitespace(xs, at)
	rest |> {
		[125, ..tail] -> (Object([]), tail, pos + 1)
		_             -> object_items(rest, pos, [])
	}
}

object_items = (
	xs: List(Int)
	at: Int
	acc: List((String, Json))
) -> (Json, List(Int), Int) ^ JsonError {
	(rest, pos) = whitespace(xs, at)
	rest |> {
		[34, ..tail] -> {
			(key, after_key, key_end) = read_string(tail, pos + 1, [])
			(colon, colon_at) = whitespace(after_key, key_end)
			colon |> {
				[58, ..after_colon] -> {
					(v, after_value, value_end) = value(
						after_colon
						colon_at + 1
					)
					(ending, end) = whitespace(after_value, value_end)
					ending |> {
						[125, ..after] -> (
							Object(list.reverse([(key, v), ..acc]))
							after
							end + 1
						)
						[44, ..after] -> object_items(
							after
							end + 1
							[(key, v), ..acc]
						)
						_ -> fail.raise(
							JsonError(end, "expected comma or closing brace")
						)
					}
				}
				_ -> fail.raise(JsonError(colon_at, "expected colon"))
			}
		}
		_ -> fail.raise(JsonError(pos, "expected object key"))
	}
}

read_string = (
	xs: List(Int)
	at: Int
	acc: List(BitArray)
) -> (String, List(Int), Int) ^ JsonError {
	xs |> {
		[] -> fail.raise(JsonError(at, "unterminated string"))
		[34, ..tail] -> {
			bit_array.to_string(bit_array.concat(list.reverse(acc))) |> {
				Ok(s)    -> (s, tail, at + 1)
				Error(_) -> fail.raise(JsonError(at, "invalid UTF-8"))
			}
		}
		[92, ..tail] -> escape(tail, at + 1, acc)
		[x, .._] ? x < 32 -> fail.raise(
			JsonError(at, "control character in string")
		)
		[x, ..tail] -> read_string(tail, at + 1, [<<x>>, ..acc])
	}
}

escape = (
	xs: List(Int)
	at: Int
	acc: List(BitArray)
) -> (String, List(Int), Int) ^ JsonError {
	xs |> {
		[117, ..tail] -> {
			(n, after, end) = hex4(tail, at + 1)
			{ n >= 55296 && n <= 56319 } |> {
				True -> after |> {
					[92, 117, ..next] -> {
						(low, rest, pos) = hex4(next, end + 2)
						{ low >= 56320 && low <= 57343 } |> {
							True -> read_string(
								rest
								pos
								[
									scalar(
										65536 +
											{ n - 55296 } * 1024 +
											low -
											56320
									)
									..acc
								]
							)
							False -> fail.raise(
								JsonError(end, "invalid surrogate pair")
							)
						}
					}
					_ -> fail.raise(JsonError(end, "missing low surrogate"))
				}
				False -> { n >= 56320 && n <= 57343 } |> {
					True  -> fail.raise(JsonError(at, "lone low surrogate"))
					False -> read_string(after, end, [scalar(n), ..acc])
				}
			}
		}
		[x, ..tail] -> {
			code = x |> {
				34  -> 34
				92  -> 92
				47  -> 47
				98  -> 8
				102 -> 12
				110 -> 10
				114 -> 13
				116 -> 9
				_   -> -1
			}
			{ code < 0 } |> {
				True  -> fail.raise(JsonError(at, "invalid escape"))
				False -> read_string(tail, at + 1, [<<code>>, ..acc])
			}
		}
		_ -> fail.raise(JsonError(at, "unterminated escape"))
	}
}

hex4 = (xs: List(Int), at: Int) -> (Int, List(Int), Int) ^ JsonError {
	xs |> {
		[a, b, c, d, ..tail] -> {
			digits = [hex(a), hex(b), hex(c), hex(d)]
			{ list.all(digits, (x) { x >= 0 }) } |> {
				True -> (
					list.fold(digits, 0, (n, x) { n * 16 + x })
					tail
					at + 4
				)
				False -> fail.raise(JsonError(at, "invalid Unicode escape"))
			}
		}
		_ -> fail.raise(JsonError(at, "short Unicode escape"))
	}
}

hex = (x: Int) -> Int {
	x |> {
		n ? n >= 48 && n <= 57  -> n - 48
		n ? n >= 65 && n <= 70  -> n - 55
		n ? n >= 97 && n <= 102 -> n - 87
		_                       -> -1
	}
}

scalar = (n: Int) -> BitArray {
	core.int_to_codepoint(n) |> {
		Ok(c)    -> <<core.from_codepoints([c]):utf8>>
		Error(_) -> panic("json: invalid scalar")
	}
}

digit = (x: Int) -> Bool { x >= 48 && x <= 57 }

number = (xs: List(Int), at: Int) -> (Json, List(Int), Int) ^ JsonError {
	(negative, start, pos) = xs |> {
		[45, ..tail] -> (True, tail, at + 1)
		_            -> (False, xs, at)
	}
	(whole, after, end) = start |> {
		[48, ..tail] -> {
			tail |> {
				[x, .._] ? digit(x) -> fail.raise(
					JsonError(pos + 1, "leading zero")
				)
				_ -> ([48], tail, pos + 1)
			}
		}
		[x, ..tail] ? x >= 49 && x <= 57 -> digits(tail, pos + 1, [x])
		_                                -> fail.raise(JsonError(pos, "expected digit"))
	}
	(fraction, rest, finish, floating) = after |> {
		[46, ..tail] -> {
			(ds, rem, p) = digits(tail, end + 1, [])
			{ ds == [] } |> {
				True  -> fail.raise(JsonError(p, "expected fraction digit"))
				False -> ([46, ..ds], rem, p, True)
			}
		}
		_ -> ([], after, end, False)
	}
	(exponent, remaining, final, is_float) = rest |> {
		[e, ..tail] ? e == 101 || e == 69 -> {
			(sign, start_exp, exp_at) = tail |> {
				[s, ..t] ? s == 43 || s == 45 -> ([s], t, finish + 2)
				_                             -> ([], tail, finish + 1)
			}
			(ds, rem, p) = digits(start_exp, exp_at, [])
			{ ds == [] } |> {
				True  -> fail.raise(JsonError(p, "expected exponent digit"))
				False -> (list.append([e, ..sign], ds), rem, p, True)
			}
		}
		_ -> ([], rest, finish, floating)
	}
	prefix = negative |> {
		True  -> [45, ..whole]
		False -> whole
	}
	num = text_bytes(list.append(list.append(prefix, fraction), exponent))
	parsed = is_float |> {
		False -> int.parse(num) |> {
			Ok(n)    -> Integer(n)
			Error(_) -> fail.raise(JsonError(at, "number out of range"))
		}
		True -> {
			float_text = { fraction == [] } |> {
				True -> text_bytes(
					list.append(list.append(prefix, [46, 48]), exponent)
				)
				False -> num
			}
			float.parse(float_text) |> {
				Ok(n)    -> Number(n)
				Error(_) -> fail.raise(JsonError(at, "number out of range"))
			}
		}
	}
	(parsed, remaining, final)
}

digits = (
	xs: List(Int)
	at: Int
	acc: List(Int)
) -> (List(Int), List(Int), Int) {
	xs |> {
		[x, ..tail] ? digit(x) -> digits(tail, at + 1, [x, ..acc])
		_                      -> (list.reverse(acc), xs, at)
	}
}

octets = (bits: BitArray) -> List(Int) {
	go = (rest, acc) {
		rest |> {
			<<x, tail:bytes>> -> go(tail, [x, ..acc])
			_                 -> list.reverse(acc)
		}
	}
	go(bits, [])
}

text_bytes = (xs: List(Int)) -> String {
	bit_array.to_string(bit_array.concat(list.map(xs, (x) { <<x>> }))) |> {
		Ok(s)    -> s
		Error(_) -> panic("json: invalid internal text")
	}
}

field = (json: Json, key: String) -> Option(Json) {
	json |> {
		Object(xs) -> list.find_map(xs, (pair) {
			(k, v) = pair
			{ k == key } |> {
				True  -> Some(v)
				False -> None
			}
		})
		_ -> None
	}
}

index = (json: Json, at: Int) -> Option(Json) {
	json |> {
		Array(xs) -> list.at(xs, at)
		_         -> None
	}
}

as_bool = (json: Json) -> Option(Bool) {
	json |> {
		Boolean(x) -> Some(x)
		_          -> None
	}
}

as_int = (json: Json) -> Option(Int) {
	json |> {
		Integer(x) -> Some(x)
		_          -> None
	}
}

as_float = (json: Json) -> Option(Float) {
	json |> {
		Number(x) -> Some(x)
		_         -> None
	}
}

as_string = (json: Json) -> Option(String) {
	json |> {
		Text(x) -> Some(x)
		_       -> None
	}
}

as_array = (json: Json) -> Option(List(Json)) {
	json |> {
		Array(x) -> Some(x)
		_        -> None
	}
}

as_object = (json: Json) -> Option(List((String, Json))) {
	json |> {
		Object(x) -> Some(x)
		_         -> None
	}
}

print = (json: Json) -> String { render(json, "", 0, False) }

pretty = (json: Json, indent: Int) -> String {
	render(json, string.repeat(" ", indent), 0, True)
}

render = (json: Json, indent: String, depth: Int, formatted: Bool) -> String {
	json |> {
		Null           -> "null"
		Boolean(True)  -> "true"
		Boolean(False) -> "false"
		Integer(x)     -> int.to_string(x)
		Number(x)      -> float.to_string(x)
		Text(x)        -> quote(x)
		Array(xs) -> collection(
			"["
			"]"
			list.map(xs, (x) { render(x, indent, depth + 1, formatted) })
			indent
			depth
			formatted
		)
		Object(xs) -> collection("{", "}", list.map(xs, (pair) {
			(k, v) = pair
			string.concat([quote(k), { formatted } |> {
				True  -> ": "
				False -> ":"
			}, render(v, indent, depth + 1, formatted)])
		}), indent, depth, formatted)
	}
}

collection = (
	open: String
	close: String
	xs: List(String)
	indent: String
	depth: Int
	formatted: Bool
) -> String {
	{ xs == [] || formatted == False } |> {
		True -> string.concat([open, string.join(xs, ","), close])
		False -> {
			pad = string.repeat(indent, depth + 1)
			string.concat([
				open
				"\n"
				pad
				string.join(xs, string.concat([",\n", pad]))
				"\n"
				string.repeat(indent, depth)
				close
			])
		}
	}
}

quote = (s: String) -> String {
	pieces = list.map(core.codepoints(s), (cp) {
		x = core.codepoint_to_int(cp)
		x |> {
			34 -> #"\""#
			92 -> #"\\"#
			8  -> #"\b"#
			9  -> #"\t"#
			10 -> #"\n"#
			12 -> #"\f"#
			13 -> #"\r"#
			n ? n < 32 -> string.concat([
				#"\u00"#
				text_bytes([hex_char(n / 16), hex_char(n % 16)])
			])
			_ -> core.from_codepoints([cp])
		}
	})
	string.concat(["\"", string.concat(pieces), "\""])
}

hex_char = (n: Int) -> Int {
	{ n < 10 } |> {
		True  -> n + 48
		False -> n + 87
	}
}
