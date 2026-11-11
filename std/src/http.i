//// HTTP/1.1 message values and incremental framing, without I/O.
(
	i/fail
	std/list
	std/string
	std/int
	std/bit_array
	std/uri
	(Option, Some, None) = std/option
)

(
	Method(..)
	Status(..)
	Headers(..)
	Request(..)
	Response(..)
	HttpError(..)
	Progress(..)
	request
	response
	feed_request
	feed_response
	feed_response_for
	feed_request_limit
	header
	header_bytes
	headers
	add_header
	serialize_request
	serialize_response
	reason
	parse_query
	build_query
	content_length
	content_type
	transfer_encoding
	host
	connection
)

content_length = "Content-Length"

content_type = "Content-Type"

transfer_encoding = "Transfer-Encoding"

host = "Host"

connection = "Connection"

Method {
	Method(String)
}

Status {
	Status(Int)
}

Headers {
	Headers(List((String, String)))
	RawHeaders(List((String, BitArray)))
}

Request {
	Request(
		method:   Method
		target:   String
		headers:  Headers
		body:     BitArray
		trailers: Headers
	)
}

Response {
	Response(
		status:   Status
		reason:   String
		headers:  Headers
		body:     BitArray
		trailers: Headers
	)
}

HttpError {
	NeedMore
	BadMessage(String)
}

Progress(A) {
	More(BitArray)
	Complete(A, BitArray)
	Invalid(String)
}

request = (method: Method, target: String) -> Request {
	Request(method, target, Headers([]), <<>>, Headers([]))
}

response = (status: Status) -> Response {
	Status(n) = status
	Response(status, reason(n), Headers([]), <<>>, Headers([]))
}

header = (hs: Headers, name: String) -> Option(String) {
	hs |> {
		RawHeaders(_) -> header_bytes(hs, name) |> {
			Some(value) -> bit_array.to_string(value) |> {
				Ok(text) -> Some(text)
				Error(_) -> None
			}
			None -> None
		}
		Headers(xs) -> list.find_map(xs, (pair) {
			(k, v) = pair
			{ lowercase(k) == lowercase(name) } |> {
				True  -> Some(v)
				False -> None
			}
		})
	}
}

header_bytes = (hs: Headers, name: String) -> Option(BitArray) {
	hs |> {
		Headers(xs) -> list.find_map(xs, (pair) {
			(k, v) = pair
			{ lowercase(k) == lowercase(name) } |> {
				True  -> Some(<<v:utf8>>)
				False -> None
			}
		})
		RawHeaders(xs) -> list.find_map(xs, (pair) {
			(k, v) = pair
			{ lowercase(k) == lowercase(name) } |> {
				True  -> Some(v)
				False -> None
			}
		})
	}
}

headers = (hs: Headers, name: String) -> List(String) {
	hs |> {
		RawHeaders(xs) -> list.filter_map(xs, (pair) {
			(k, v) = pair
			{ lowercase(k) == lowercase(name) } |> {
				True -> bit_array.to_string(v) |> {
					Ok(value) -> Some(value)
					Error(_)  -> None
				}
				False -> None
			}
		})
		Headers(xs) -> list.map(list.filter(xs, (pair) {
			(k, _) = pair
			lowercase(k) == lowercase(name)
		}), (pair) {
			(_, v) = pair
			v
		})
	}
}

add_header = (hs: Headers, name: String, value: String) -> Headers {
	hs |> {
		Headers(xs)    -> Headers(list.append(xs, [(name, value)]))
		RawHeaders(xs) -> RawHeaders(list.append(xs, [(name, <<value:utf8>>)]))
	}
}

parse_query = (
	query: String
) -> Result(List((String, String)), uri.CodecError) { uri.parse_query(query) }

build_query = (pairs: List((String, String))) -> String {
	uri.build_query(pairs)
}

feed_request = (buffer: BitArray, bytes: BitArray) -> Progress(Request) {
	input = bit_array.concat([buffer, bytes])
	fail.catch(() { parse_request(octets(input)) }) |> {
		Ok((message, rest))    -> Complete(message, bits(rest))
		Error(NeedMore)        -> More(input)
		Error(BadMessage(why)) -> Invalid(why)
	}
}

feed_response = (buffer: BitArray, bytes: BitArray) -> Progress(Response) {
	input = bit_array.concat([buffer, bytes])
	fail.catch(() { parse_response(octets(input)) }) |> {
		Ok((message, rest))    -> Complete(message, bits(rest))
		Error(NeedMore)        -> More(input)
		Error(BadMessage(why)) -> Invalid(why)
	}
}

/// Parse a response with its request method and whether the connection ended.
feed_response_for = (
	buffer: BitArray
	bytes: BitArray
	method: Method
	eof: Bool
	head_limit: Int
) -> Progress(Response) {
	input = bit_array.concat([buffer, bytes])
	fail.catch(() {
		check_head(octets(input), head_limit)
		parse_response_for(octets(input), method, eof)
	}) |> {
		Ok((message, rest))    -> Complete(message, bits(rest))
		Error(NeedMore)        -> More(input)
		Error(BadMessage(why)) -> Invalid(why)
	}
}

feed_request_limit = (
	buffer: BitArray
	bytes: BitArray
	head_limit: Int
) -> Progress(Request) {
	input = bit_array.concat([buffer, bytes])
	fail.catch(() {
		check_head(octets(input), head_limit)
		parse_request(octets(input))
	}) |> {
		Ok((message, rest))    -> Complete(message, bits(rest))
		Error(NeedMore)        -> More(input)
		Error(BadMessage(why)) -> Invalid(why)
	}
}

check_head = (xs: List(Int), limit: Int) -> Nil ? HttpError {
	go = (rest: List(Int), count: Int) -> Nil ? HttpError {
		{ count > limit } |> {
			True -> fail.raise(BadMessage("head too large"))
			False -> rest |> {
				[13, 10, 13, 10, .._] -> require(
					count + 4 <= limit
					"head too large"
				)
				[_, ..tail] -> go(tail, count + 1)
				[]          -> Nil
			}
		}
	}
	go(xs, 0)
}

parse_request = (xs: List(Int)) -> (Request, List(Int)) ? HttpError {
	(line, rest) = line(xs)
	parts = string.split(text(line), " ")
	parts |> {
		[method, target, "HTTP/1.1"] -> {
			require(
				method != "" && list.all(octets(<<method:utf8>>), token)
				"invalid method"
			)
			require(
				target != "" && list.all(octets(<<target:utf8>>), (x) {
					x > 32 && x != 127
				})
				"invalid target"
			)
			(hs, after) = read_headers(rest, [])
			(body, trailers, tail) = read_body(hs, after)
			(Request(Method(method), target, hs, body, trailers), tail)
		}
		_ -> fail.raise(BadMessage("invalid request line"))
	}
}

parse_response = (xs: List(Int)) -> (Response, List(Int)) ? HttpError {
	parse_response_for(xs, Method("GET"), True)
}

parse_response_for = (
	xs: List(Int)
	method: Method
	eof: Bool
) -> (Response, List(Int)) ? HttpError {
	(first, rest) = line(xs)
	raw = first |> {
		[72, 84, 84, 80, 47, 49, 46, 49, 32, a, b, c, 32, ..phrase] ? digit(
			a
		) &&
			digit(b) &&
			digit(c) -> (a, b, c, phrase)
		_ -> fail.raise(BadMessage("invalid status line"))
	}
	(a, b, c, phrase) = raw
	require(list.all(phrase, field_value), "invalid reason phrase")
	(hs, after) = read_headers(rest, [])
	validate_framing(hs)
	code = { a - 48 } * 100 + { b - 48 } * 10 + c - 48
	Method(verb) = method
	(body, trailers, tail) = {
		verb == "HEAD" ||
			code < 200 ||
			code == 204 ||
			code == 304 ||
			verb == "CONNECT" && code >= 200 && code < 300
	} |> {
		True -> (<<>>, Headers([]), after)
		False -> {
			headers(hs, "content-length") == [] &&
				list.last(transfer_codings(hs)) != Some("chunked")
		} |> {
			True -> eof |> {
				True  -> (bits(after), Headers([]), [])
				False -> fail.raise(NeedMore)
			}
			False -> read_body(hs, after)
		}
	}
	(Response(Status(code), text(phrase), hs, body, trailers), tail)
}

read_headers = (
	xs: List(Int)
	acc: List((String, BitArray))
) -> (Headers, List(Int)) ? HttpError {
	(head, rest) = line(xs)
	head |> {
		[] -> (make_headers(list.reverse(acc)), rest)
		_ -> {
			(name, value, found) = cut(head, 58)
			require(
				found && name != [] && list.all(name, token)
				"invalid header name"
			)
			require(list.all(value, field_value), "invalid header value")
			read_headers(rest, [(text(name), bits(trim(value))), ..acc])
		}
	}
}

make_headers = (xs: List((String, BitArray))) -> Headers {
	values = list.map(xs, (pair) {
		(name, value) = pair
		bit_array.to_string(value) |> {
			Ok(text) -> Some((name, text))
			Error(_) -> None
		}
	})
	{ list.all(values, (x) { x != None }) } |> {
		True  -> Headers(list.filter_map(values, (x) { x }))
		False -> RawHeaders(xs)
	}
}

text_unchecked = (xs: List(Int)) -> String {
	bit_array.to_string(bits(xs)) |> {
		Ok(value) -> value
		Error(_)  -> panic("http: invalid internal text")
	}
}

validate_framing = (hs: Headers) -> Nil ? HttpError {
	length = header_bytes(hs, "content-length")
	transfer = header_bytes(hs, "transfer-encoding")
	require(
		length == None || transfer == None
		"both content-length and transfer-encoding"
	)
	require(
		length == None || header(hs, "content-length") != None
		"invalid content-length"
	)
	require(
		transfer == None || header(hs, "transfer-encoding") != None
		"invalid transfer-encoding"
	)
	list.each(raw_pairs(hs), (pair) {
		(name, value) = pair
		{
			lowercase(name) == "content-length" ||
				lowercase(name) == "transfer-encoding"
		} |> {
			True -> require(bit_array.to_string(value) |> {
				Ok(_)    -> True
				Error(_) -> False
			}, "invalid framing field")
			False -> Nil
		}
	})
	lengths = list.concat(list.map(headers(hs, "content-length"), (value) {
		list.map(string.split(value, ","), (part) {
			parse_length(text_unchecked(trim(octets(<<part:utf8>>))))
		})
	}))
	lengths |> {
		[] -> Nil
		[first, ..others] -> require(
			list.all(others, (x) { x == first })
			"differing content-length"
		)
	}
	transfer_codings(hs)
	Nil
}

read_body = (
	hs: Headers
	xs: List(Int)
) -> (BitArray, Headers, List(Int)) ? HttpError {
	validate_framing(hs)
	lengths = list.concat(list.map(headers(hs, "content-length"), (value) {
		list.map(string.split(value, ","), (part) {
			text_unchecked(trim(octets(<<part:utf8>>)))
		})
	}))
	transfers = headers(hs, "transfer-encoding")
	transfers |> {
		[] -> lengths |> {
			[] -> (<<>>, Headers([]), xs)
			[n, ..others] -> {
				size = parse_length(n)
				list.each(others, (value) {
					require(
						parse_length(value) == size
						"differing content-length"
					)
				})
				(part, rest) = take(xs, size, [])
				(bits(part), Headers([]), rest)
			}
		}
		_ -> {
			codings = transfer_codings(hs)
			require(
				list.last(codings) == Some("chunked")
				"invalid transfer-encoding"
			)
			chunks(xs, [])
		}
	}
}

transfer_codings = (hs: Headers) -> List(String) ? HttpError {
	codings = list.concat(list.map(headers(hs, "transfer-encoding"), (value) {
		list.map(string.split(value, ","), (part) {
			lowercase(text_unchecked(trim(octets(<<part:utf8>>))))
		})
	}))
	list.each(codings, (coding) {
		raw = octets(<<coding:utf8>>)
		require(raw != [] && list.all(raw, token), "invalid transfer-encoding")
	})
	require(
		list.count(codings, (coding) { coding == "chunked" }) <= 1
		"repeated chunked encoding"
	)
	require(
		list.contains(codings, "chunked") == False ||
			list.last(codings) == Some("chunked")
		"chunked encoding is not final"
	)
	codings
}

parse_length = (value: String) -> Int ? HttpError {
	raw = octets(<<value:utf8>>)
	require(raw != [] && list.all(raw, digit), "invalid content-length")
	int.parse(value) |> {
		Ok(size) -> size
		Error(_) -> fail.raise(BadMessage("invalid content-length"))
	}
}

chunks = (
	xs: List(Int)
	acc: List(BitArray)
) -> (BitArray, Headers, List(Int)) ? HttpError {
	(size_line, rest) = line(xs)
	(size_raw, extension, has_extension) = cut(size_line, 59)
	size_bytes = list.reverse(list.drop_while(list.reverse(size_raw), space))
	require(has_extension || size_bytes == size_raw, "invalid chunk size")
	{ has_extension } |> {
		True  -> validate_extensions(extension)
		False -> Nil
	}
	require(
		size_bytes != [] && list.all(size_bytes, (x) { hex(x) >= 0 })
		"invalid chunk size"
	)
	size = list.fold(size_bytes, 0, (n, x) { n * 16 + hex(x) })
	require(size >= 0, "invalid chunk size")
	{ size == 0 } |> {
		True -> {
			(trailers, tail) = read_headers(rest, [])
			require(
				header_bytes(trailers, "content-length") == None &&
					header_bytes(trailers, "transfer-encoding") == None &&
					header_bytes(trailers, "host") == None
				"invalid trailer field"
			)
			(bit_array.concat(list.reverse(acc)), trailers, tail)
		}
		False -> {
			(part, tail) = take(rest, size, [])
			tail |> {
				[13, 10, ..after] -> chunks(after, [bits(part), ..acc])
				[]                -> fail.raise(NeedMore)
				[13]              -> fail.raise(NeedMore)
				_                 -> fail.raise(BadMessage("invalid chunk terminator"))
			}
		}
	}
}

token_prefix = (xs: List(Int), acc: List(Int)) -> (List(Int), List(Int)) {
	xs |> {
		[x, ..tail] ? token(x) -> token_prefix(tail, [x, ..acc])
		_                      -> (list.reverse(acc), xs)
	}
}

validate_extensions = (xs: List(Int)) -> Nil ? HttpError {
	(name, after_name) = token_prefix(list.drop_while(xs, space), [])
	rest = list.drop_while(after_name, space)
	require(name != [], "invalid chunk extension")
	rest |> {
		[]           -> require(rest == after_name, "invalid chunk extension")
		[59, ..tail] -> validate_extensions(tail)
		[61, ..raw] -> {
			tail = list.drop_while(raw, space)
			after_value = tail |> {
				[34, ..quoted] -> extension_quoted(quoted)
				_ -> {
					(value, remaining) = token_prefix(tail, [])
					require(value != [], "invalid chunk extension")
					remaining
				}
			}
			list.drop_while(after_value, space) |> {
				[]           -> require(after_value == [], "invalid chunk extension")
				[59, ..next] -> validate_extensions(next)
				_            -> fail.raise(BadMessage("invalid chunk extension"))
			}
		}
		_ -> fail.raise(BadMessage("invalid chunk extension"))
	}
}

extension_quoted = (xs: List(Int)) -> List(Int) ? HttpError {
	xs |> {
		[34, ..tail]                            -> tail
		[92, x, ..tail] ? field_value(x)        -> extension_quoted(tail)
		[x, ..tail] ? field_value(x) && x != 92 -> extension_quoted(tail)
		_                                       -> fail.raise(BadMessage("invalid chunk extension"))
	}
}

line = (xs: List(Int)) -> (List(Int), List(Int)) ? HttpError {
	go = (
		rest: List(Int)
		acc: List(Int)
	) -> (List(Int), List(Int)) ? HttpError {
		rest |> {
			[]               -> fail.raise(NeedMore)
			[13]             -> fail.raise(NeedMore)
			[13, 10, ..tail] -> (list.reverse(acc), tail)
			[13, .._]        -> fail.raise(BadMessage("invalid line ending"))
			[10, .._]        -> fail.raise(BadMessage("invalid line ending"))
			[x, ..tail]      -> go(tail, [x, ..acc])
		}
	}
	go(xs, [])
}

take = (
	xs: List(Int)
	n: Int
	acc: List(Int)
) -> (List(Int), List(Int)) ? HttpError {
	{ n == 0 } |> {
		True -> (list.reverse(acc), xs)
		False -> xs |> {
			[x, ..tail] -> take(tail, n - 1, [x, ..acc])
			[]          -> fail.raise(NeedMore)
		}
	}
}

cut = (xs: List(Int), separator: Int) -> (List(Int), List(Int), Bool) {
	go = (rest, acc) {
		rest |> {
			[]                           -> (list.reverse(acc), [], False)
			[x, ..tail] ? x == separator -> (list.reverse(acc), tail, True)
			[x, ..tail]                  -> go(tail, [x, ..acc])
		}
	}
	go(xs, [])
}

trim = (xs: List(Int)) -> List(Int) {
	list.reverse(
		list.drop_while(list.reverse(list.drop_while(xs, space)), space)
	)
}

space = (x: Int) -> Bool { x == 32 || x == 9 }

field_value = (x: Int) -> Bool { x == 9 || x >= 32 && x != 127 }

digit = (x: Int) -> Bool { x >= 48 && x <= 57 }

hex = (x: Int) -> Int {
	x |> {
		n ? n >= 48 && n <= 57  -> n - 48
		n ? n >= 65 && n <= 70  -> n - 55
		n ? n >= 97 && n <= 102 -> n - 87
		_                       -> -1
	}
}

token = (x: Int) -> Bool {
	digit(x) ||
		x >= 65 && x <= 90 ||
		x >= 97 && x <= 122 ||
		list.contains(
			[33, 35, 36, 37, 38, 39, 42, 43, 45, 46, 94, 95, 96, 124, 126]
			x
		)
}

require = (ok: Bool, why: String) -> Nil ? HttpError {
	ok |> {
		True  -> Nil
		False -> fail.raise(BadMessage(why))
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

bits = (xs: List(Int)) -> BitArray {
	bit_array.concat(list.map(xs, (x) { <<x>> }))
}

text = (xs: List(Int)) -> String ? HttpError {
	bit_array.to_string(bits(xs)) |> {
		Ok(s)    -> s
		Error(_) -> fail.raise(BadMessage("invalid UTF-8"))
	}
}

serialize_request = (req: Request) -> BitArray {
	Method(method) = req.method
	serialize(
		string.concat([method, " ", req.target, " HTTP/1.1\r\n"])
		req.headers
		req.body
		req.trailers
	)
}

serialize_response = (res: Response) -> BitArray {
	Status(status) = res.status
	serialize(
		string.concat([
			"HTTP/1.1 "
			int.to_string(status)
			" "
			res.reason
			"\r\n"
		])
		res.headers
		res.body
		res.trailers
	)
}

serialize = (
	first: String
	hs: Headers
	body: BitArray
	trailers: Headers
) -> BitArray {
	xs = raw_pairs(hs)
	chunked = header(hs, "transfer-encoding") |> {
		Some(v) -> list.last(list.map(string.split(v, ","), (part) {
			lowercase(text_unchecked(trim(octets(<<part:utf8>>))))
		})) ==
			Some("chunked")
		None -> False
	}
	with_length = {
		chunked == False &&
			header(hs, "content-length") == None &&
			bit_array.bit_size(body) > 0
	} |> {
		True -> list.append(
			xs
			[
				(
					"Content-Length"
					<<int.to_string(bit_array.bit_size(body) / 8):utf8>>
				)
			]
		)
		False -> xs
	}
	lines = header_lines(with_length)
	data = chunked |> {
		False -> body
		True -> {
			end = header_lines(raw_pairs(trailers))
			prefix = { bit_array.bit_size(body) == 0 } |> {
				True -> <<>>
				False -> bit_array.concat([
					<<hex_string(bit_array.bit_size(body) / 8):utf8>>
					<<"\r\n":utf8>>
					body
					<<"\r\n":utf8>>
				])
			}
			bit_array.concat([prefix, <<"0\r\n":utf8>>, end, <<"\r\n":utf8>>])
		}
	}
	bit_array.concat([<<first:utf8>>, lines, <<"\r\n":utf8>>, data])
}

raw_pairs = (hs: Headers) -> List((String, BitArray)) {
	hs |> {
		Headers(xs) -> list.map(xs, (pair) {
			(k, v) = pair
			(k, <<v:utf8>>)
		})
		RawHeaders(xs) -> xs
	}
}

header_lines = (xs: List((String, BitArray))) -> BitArray {
	bit_array.concat(list.map(xs, (pair) {
		(k, v) = pair
		bit_array.concat([<<k:utf8>>, <<": ":utf8>>, v, <<"\r\n":utf8>>])
	}))
}

reason = (status: Int) -> String {
	status |> {
		100 -> "Continue"
		101 -> "Switching Protocols"
		102 -> "Processing"
		103 -> "Early Hints"
		200 -> "OK"
		201 -> "Created"
		202 -> "Accepted"
		203 -> "Non-Authoritative Information"
		204 -> "No Content"
		205 -> "Reset Content"
		206 -> "Partial Content"
		207 -> "Multi-Status"
		208 -> "Already Reported"
		226 -> "IM Used"
		300 -> "Multiple Choices"
		301 -> "Moved Permanently"
		302 -> "Found"
		303 -> "See Other"
		304 -> "Not Modified"
		307 -> "Temporary Redirect"
		308 -> "Permanent Redirect"
		400 -> "Bad Request"
		401 -> "Unauthorized"
		402 -> "Payment Required"
		403 -> "Forbidden"
		404 -> "Not Found"
		405 -> "Method Not Allowed"
		406 -> "Not Acceptable"
		407 -> "Proxy Authentication Required"
		408 -> "Request Timeout"
		409 -> "Conflict"
		410 -> "Gone"
		411 -> "Length Required"
		412 -> "Precondition Failed"
		413 -> "Content Too Large"
		414 -> "URI Too Long"
		415 -> "Unsupported Media Type"
		416 -> "Range Not Satisfiable"
		417 -> "Expectation Failed"
		421 -> "Misdirected Request"
		422 -> "Unprocessable Content"
		423 -> "Locked"
		424 -> "Failed Dependency"
		425 -> "Too Early"
		426 -> "Upgrade Required"
		428 -> "Precondition Required"
		429 -> "Too Many Requests"
		431 -> "Request Header Fields Too Large"
		451 -> "Unavailable For Legal Reasons"
		500 -> "Internal Server Error"
		501 -> "Not Implemented"
		502 -> "Bad Gateway"
		503 -> "Service Unavailable"
		504 -> "Gateway Timeout"
		505 -> "HTTP Version Not Supported"
		506 -> "Variant Also Negotiates"
		507 -> "Insufficient Storage"
		508 -> "Loop Detected"
		510 -> "Not Extended"
		511 -> "Network Authentication Required"
		_   -> ""
	}
}

hex_string = (n: Int) -> String {
	chars = "0123456789abcdef"
	go = (x: Int, acc: String) -> String {
		c = string.at(chars, x % 16) |> {
			Some(v) -> v
			None    -> panic("http: invalid hex digit")
		}
		{ x < 16 } |> {
			True  -> c <> acc
			False -> go(x / 16, c <> acc)
		}
	}
	go(n, "")
}

lowercase = (s: String) -> String {
	raw = list.map(octets(<<s:utf8>>), (x) {
		{ x >= 65 && x <= 90 } |> {
			True  -> x + 32
			False -> x
		}
	})
	bit_array.to_string(bits(raw)) |> {
		Ok(value) -> value
		Error(_)  -> panic("http: invalid internal text")
	}
}
