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
	header
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
	Headers(xs) = hs
	list.find_map(xs, (pair) {
		(k, v) = pair
		{ lowercase(k) == lowercase(name) } |> {
			True  -> Some(v)
			False -> None
		}
	})
}

headers = (hs: Headers, name: String) -> List(String) {
	Headers(xs) = hs
	list.map(list.filter(xs, (pair) {
		(k, _) = pair
		lowercase(k) == lowercase(name)
	}), (pair) {
		(_, v) = pair
		v
	})
}

add_header = (hs: Headers, name: String, value: String) -> Headers {
	Headers(xs) = hs
	Headers(list.append(xs, [(name, value)]))
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
	code = { a - 48 } * 100 + { b - 48 } * 10 + c - 48
	(body, trailers, tail) = { code < 200 || code == 204 || code == 304 } |> {
		True  -> (<<>>, Headers([]), after)
		False -> read_body(hs, after)
	}
	(Response(Status(code), text(phrase), hs, body, trailers), tail)
}

read_headers = (
	xs: List(Int)
	acc: List((String, String))
) -> (Headers, List(Int)) ? HttpError {
	(head, rest) = line(xs)
	head |> {
		[] -> (Headers(list.reverse(acc)), rest)
		_ -> {
			(name, value, found) = cut(head, 58)
			require(
				found && name != [] && list.all(name, token)
				"invalid header name"
			)
			require(list.all(value, field_value), "invalid header value")
			read_headers(rest, [(text(name), text(trim(value))), ..acc])
		}
	}
}

read_body = (
	hs: Headers
	xs: List(Int)
) -> (BitArray, Headers, List(Int)) ? HttpError {
	lengths = headers(hs, "content-length")
	transfers = headers(hs, "transfer-encoding")
	require(
		lengths == [] || transfers == []
		"both content-length and transfer-encoding"
	)
	transfers |> {
		[] -> lengths |> {
			[] -> (<<>>, Headers([]), xs)
			[n] -> {
				raw = octets(<<n:utf8>>)
				require(
					raw != [] && list.all(raw, digit)
					"invalid content-length"
				)
				int.parse(n) |> {
					Ok(size) -> {
						require(size >= 0, "invalid content-length")
						(part, rest) = take(xs, size, [])
						(bits(part), Headers([]), rest)
					}
					Error(_) -> fail.raise(BadMessage("invalid content-length"))
				}
			}
			_ -> fail.raise(BadMessage("repeated content-length"))
		}
		[encoding] -> {
			require(
				lowercase(encoding) == "chunked"
				"invalid transfer-encoding"
			)
			chunks(xs, [])
		}
		_ -> fail.raise(BadMessage("invalid transfer-encoding"))
	}
}

chunks = (
	xs: List(Int)
	acc: List(BitArray)
) -> (BitArray, Headers, List(Int)) ? HttpError {
	(size_line, rest) = line(xs)
	(size_bytes, _, _) = cut(size_line, 59)
	require(
		size_bytes != [] && list.all(size_bytes, (x) { hex(x) >= 0 })
		"invalid chunk size"
	)
	size = list.fold(size_bytes, 0, (n, x) { n * 16 + hex(x) })
	require(size >= 0, "invalid chunk size")
	{ size == 0 } |> {
		True -> {
			(trailers, tail) = read_headers(rest, [])
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
	Headers(xs) = hs
	chunked = header(hs, "transfer-encoding") |> {
		Some(v) -> lowercase(v) == "chunked"
		None    -> False
	}
	with_length = {
		chunked == False &&
			header(hs, "content-length") == None &&
			bit_array.bit_size(body) > 0
	} |> {
		True -> list.append(
			xs
			[("Content-Length", int.to_string(bit_array.bit_size(body) / 8))]
		)
		False -> xs
	}
	lines = string.concat(list.map(with_length, (pair) {
		(k, v) = pair
		string.concat([k, ": ", v, "\r\n"])
	}))
	data = chunked |> {
		False -> body
		True -> {
			Headers(ts) = trailers
			end = string.concat(list.map(ts, (pair) {
				(k, v) = pair
				string.concat([k, ": ", v, "\r\n"])
			}))
			prefix = { bit_array.bit_size(body) == 0 } |> {
				True -> <<>>
				False -> bit_array.concat([
					<<hex_string(bit_array.bit_size(body) / 8):utf8>>
					<<"\r\n":utf8>>
					body
					<<"\r\n":utf8>>
				])
			}
			bit_array.concat([
				prefix
				<<"0\r\n":utf8>>
				<<end:utf8>>
				<<"\r\n":utf8>>
			])
		}
	}
	bit_array.concat([<<first:utf8>>, <<lines:utf8>>, <<"\r\n":utf8>>, data])
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
