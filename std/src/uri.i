//// Strict ASCII RFC 3986 references, stored with their original encoded spelling.
//// Syntax validation does not impose DNS, scheme or port-range policy. Generic
//// percent decoding and query forms are separate operations. All public failures
//// are Result values. Parse offsets name bytes of the whole input; build and
//// resolve offsets name bytes of the rejected field. Codec offsets name input
//// bytes, including the whole query for forms. Scans and assembly are linear.

(
	i/fail
	std/list
	std/string
	std/bit_array
	(Option, Some, None) = std/option
)

(
	Uri
	Parts(..)
	Authority(..)
	Host(..)
	UriPart(..)
	UriProblem(..)
	UriError(..)
	CodecError(..)
	parse
	build
	parts
	to_string
	has_scheme
	resolve
	encode_component
	encode_bytes
	decode_component
	decode_bytes
	parse_query
	build_query
)

Uri {
	Uri(Parts)
}

Parts {
	Parts(
		scheme:    Option(String)
		authority: Option(Authority)
		path:      String
		query:     Option(String)
		fragment:  Option(String)
	)
}

Authority {
	Authority(userinfo: Option(String), host: Host, port: Option(String))
}

Host {
	RegName(String)
	IpLiteral(String)
}

UriPart {
	WholePart
	SchemePart
	AuthorityPart
	UserInfoPart
	HostPart
	PortPart
	PathPart
	QueryPart
	FragmentPart
}

UriProblem {
	BadCharacter
	BadPercent
	BadScheme
	BadAuthority
	BadHost
	BadPort
	BadPath
	RelativeBase
}

UriError {
	UriError(part: UriPart, offset: Int, problem: UriProblem)
}

CodecError {
	BadEscape(offset: Int)
	NonAscii(offset: Int)
	InvalidUtf8
	UnalignedBits
}

/// Parses without repairing, decoding or normalizing any component.
parse = (text: String) -> Result(Uri, UriError) {
	fail.catch(() {
		raw = bytes(text)
		lexical(raw, WholePart)
		(before_fragment, fragment_bytes, fragment_present) = cut(raw, 35)
		fragment_start = list.length(before_fragment) + 1
		(hier, query_bytes, query_present) = cut(before_fragment, 63)
		query_start = list.length(hier) + 1
		(prefix, after_colon, colon) = cut(hier, 58)
		(scheme, rest, start) = { colon && valid_scheme(prefix) } |> {
			True -> (
				Some(text_bytes(prefix))
				after_colon
				list.length(prefix) + 1
			)
			False -> (None, hier, 0)
		}
		(authority, path_bytes, path_start, locations) = rest |> {
			[47, 47, ..xs] -> {
				(a, path_tail, slash) = cut(xs, 47)
				astart = start + 2
				(value, offsets) = read_authority(a, astart)
				path = slash |> {
					True  -> [47, ..path_tail]
					False -> []
				}
				(Some(value), path, astart + list.length(a), offsets)
			}
			_ -> (None, rest, start, [])
		}
		p = Parts(
			scheme:    scheme
			authority: authority
			path:      text_bytes(path_bytes)
			query:     optional_text(query_bytes, query_present)
			fragment:  optional_text(fragment_bytes, fragment_present)
		)
		offsets = list.append(
			locations
			[
				(SchemePart, 0)
				(PathPart, path_start)
				(QueryPart, query_start)
				(FragmentPart, fragment_start)
			]
		)
		validated(p, True) |> {
			Ok(u) -> u
			Error(UriError(part = part, offset = offset, problem = problem)) -> {
				origin = list.fold(offsets, 0, (acc, pair) {
					(kind, at) = pair
					{ kind == part } |> {
						True  -> at
						False -> acc
					}
				})
				fail.raise(
					UriError(
						part:    part
						offset:  origin + offset
						problem: problem
					)
				)
			}
		}
	})
}

/// Validates raw candidate fields and their cross-field grammar constraints.
build = (parts: Parts) -> Result(Uri, UriError) { validated(parts, False) }

/// Projects the original raw fields in constant field work.
parts = (uri: Uri) -> Parts {
	Uri(p) = uri
	p
}

/// Reassembles raw spelling, preserving missing versus empty delimiters.
to_string = (uri: Uri) -> String {
	p = parts(uri)
	authority = p.authority |> {
		None -> ""
		Some(a) -> {
			host = a.host |> {
				RegName(h)   -> h
				IpLiteral(h) -> string.concat(["[", h, "]"])
			}
			string.concat([
				"//"
				delimited(a.userinfo, "", "@")
				host
				delimited(a.port, ":", "")
			])
		}
	}
	string.concat([
		delimited(p.scheme, "", ":")
		authority
		p.path
		delimited(p.query, "?", "")
		delimited(p.fragment, "#", "")
	])
}

has_scheme = (uri: Uri) -> Bool { parts(uri).scheme != None }

/// Strict RFC 5.2 resolution over encoded paths, followed by build validation.
resolve = (base: Uri, reference: Uri) -> Result(Uri, UriError) {
	b = parts(base)
	r = parts(reference)
	b.scheme |> {
		None -> Error(
			UriError(part: WholePart, offset: 0, problem: RelativeBase)
		)
		Some(_) -> {
			candidate = r.scheme |> {
				Some(_) -> Parts(..r, path: remove_dots(r.path))
				None -> r.authority |> {
					Some(_) -> Parts(
						..r
						scheme: b.scheme
						path:   remove_dots(r.path)
					)
					None -> {
						(path, query) = r.path |> {
							"" -> (b.path, r.query |> {
								None -> b.query
								q    -> q
							})
							"/" <> _ -> (remove_dots(r.path), r.query)
							_        -> (remove_dots(merge_path(b, r.path)), r.query)
						}
						Parts(
							scheme:    b.scheme
							authority: b.authority
							path:      path
							query:     query
							fragment:  r.fragment
						)
					}
				}
			}
			build(candidate)
		}
	}
}

/// UTF-8 percent encoding, preserving only the unreserved ASCII alphabet.
encode_component = (text: String) -> String {
	encode_octets(bytes(text), False)
}

encode_bytes = (bytes: BitArray) -> Result(String, CodecError) {
	{ bit_array.bit_size(bytes) % 8 == 0 } |> {
		True  -> Ok(encode_octets(octets(bytes), False))
		False -> Error(UnalignedBits)
	}
}

/// Decodes once, leaves plus unchanged, then validates UTF-8.
decode_component = (text: String) -> Result(String, CodecError) {
	fail.catch(() { decode_text(text, False) })
}

/// Decodes arbitrary octets; literal ASCII has no component grammar restriction.
decode_bytes = (text: String) -> Result(BitArray, CodecError) {
	fail.catch(() { decode_raw(bytes(text), False) })
}

/// Ordered query form pairs. Empty pieces are skipped; semicolons are data.
parse_query = (query: String) -> Result(List((String, String)), CodecError) {
	fail.catch(() {
		go = (
			rest: List(Int)
			offset: Int
			acc: List((String, String))
		) -> List((String, String)) ^ CodecError {
			(piece, tail, more) = cut(rest, 38)
			next = { piece == [] } |> {
				True -> acc
				False -> {
					(key, value, equal) = cut(piece, 61)
					decode_at = (input, origin) {
						fail.catch(() {
							decode_text(text_bytes(input), True)
						}) |> {
							Ok(x) -> x
							Error(BadEscape(offset = offset)) -> fail.raise(
								BadEscape(offset: origin + offset)
							)
							Error(NonAscii(offset = offset)) -> fail.raise(
								NonAscii(offset: origin + offset)
							)
							Error(e) -> fail.raise(e)
						}
					}
					k = decode_at(key, offset)
					vstart = offset + list.length(key) + 1
					v = equal |> {
						True  -> decode_at(value, vstart)
						False -> ""
					}
					[(k, v), ..acc]
				}
			}
			more |> {
				True  -> go(tail, offset + list.length(piece) + 1, next)
				False -> list.reverse(next)
			}
		}
		go(bytes(query), 0, [])
	})
}

/// Encodes UTF-8 form pairs without sorting, always including equals signs.
build_query = (pairs: List((String, String))) -> String {
	string.join(list.map(pairs, (pair) {
		(key, value) = pair
		string.concat([
			encode_octets(bytes(key), True)
			"="
			encode_octets(bytes(value), True)
		])
	}), "&")
}

validated = (p: Parts, parsing: Bool) -> Result(Uri, UriError) {
	fail.catch(() {
		validate(p, parsing)
		Uri(p)
	})
}

// Parse validates complete components in order after its whole-input scan.
// Build scans all fields, then checks their grammar, then cross-field rules.
validate = (p: Parts, parsing: Bool) -> Nil ^ UriError {
	scan_option(p.scheme, SchemePart)
	p.authority |> {
		None -> Nil
		Some(a) -> {
			scan_option(a.userinfo, UserInfoPart)
			lexical(bytes(host_text(a.host)), HostPart)
			scan_option(a.port, PortPart)
		}
	}
	lexical(bytes(p.path), PathPart)
	scan_option(p.query, QueryPart)
	scan_option(p.fragment, FragmentPart)
	p.scheme |> {
		None -> Nil
		Some(s) -> {
			xs = bytes(s)
			xs |> {
				[] -> problem(SchemePart, 0, BadScheme)
				[x, ..tail] -> {
					require(alpha(x), SchemePart, 0, BadScheme)
					grammar(tail, SchemePart, BadScheme, scheme_char, 1)
				}
			}
		}
	}
	p.authority |> {
		None -> Nil
		Some(a) -> {
			grammar_option(a.userinfo, UserInfoPart, BadAuthority, user_char)
			a.host |> {
				RegName(h)   -> grammar(bytes(h), HostPart, BadHost, reg_char, 0)
				IpLiteral(h) -> require(valid_ip(h), HostPart, 0, BadHost)
			}
			grammar_option(a.port, PortPart, BadPort, digit)
		}
	}
	parsing |> {
		True  -> path_constraints(p)
		False -> Nil
	}
	grammar(bytes(p.path), PathPart, BadPath, path_char, 0)
	grammar_option(p.query, QueryPart, BadCharacter, query_char)
	grammar_option(p.fragment, FragmentPart, BadCharacter, query_char)
	parsing |> {
		True  -> Nil
		False -> path_constraints(p)
	}
}

path_constraints = (p: Parts) -> Nil ^ UriError {
	path_bytes = bytes(p.path)
	p.authority |> {
		Some(_) -> path_bytes |> {
			[]        -> Nil
			[47, .._] -> Nil
			_         -> problem(PathPart, 0, BadPath)
		}
		None -> {
			path_bytes |> {
				[47, 47, .._] -> problem(PathPart, 0, BadPath)
				_             -> Nil
			}
			p.scheme |> {
				Some(_) -> Nil
				None -> {
					(first, _, _) = cut(path_bytes, 47)
					grammar(first, PathPart, BadPath, no_colon, 0)
				}
			}
		}
	}
}

problem = (part: UriPart, offset: Int, why: UriProblem) -> Never ^ UriError {
	fail.raise(UriError(part: part, offset: offset, problem: why))
}

require = (
	ok: Bool
	part: UriPart
	offset: Int
	why: UriProblem
) -> Nil ^ UriError {
	ok |> {
		True  -> Nil
		False -> problem(part, offset, why)
	}
}

scan_option = (s: Option(String), part: UriPart) -> Nil ^ UriError {
	s |> {
		None       -> Nil
		Some(text) -> lexical(bytes(text), part)
	}
}

lexical = (xs: List(Int), part: UriPart) -> Nil ^ UriError {
	go = (rest: List(Int), at: Int) -> Nil ^ UriError {
		rest |> {
			[] -> Nil
			[x, ..tail] -> { x == 37 } |> {
				True -> tail |> {
					[a, b, ..after] -> {
						require(
							hex(a) >= 0 && hex(b) >= 0
							part
							at
							BadPercent
						)
						go(after, at + 3)
					}
					_ -> problem(part, at, BadPercent)
				}
				False -> {
					require(uri_char(x), part, at, BadCharacter)
					go(tail, at + 1)
				}
			}
		}
	}
	go(xs, 0)
}

grammar_option = (
	text: Option(String)
	part: UriPart
	why: UriProblem
	allowed: (Int) -> Bool
) -> Nil ^ UriError {
	text |> {
		None    -> Nil
		Some(s) -> grammar(bytes(s), part, why, allowed, 0)
	}
}

grammar = (
	xs: List(Int)
	part: UriPart
	why: UriProblem
	allowed: (Int) -> Bool
	at: Int
) -> Nil ^ UriError {
	xs |> {
		[] -> Nil
		[x, ..tail] -> {
			require(allowed(x), part, at, why)
			{ x == 37 } |> {
				True -> tail |> {
					[a, b, ..after] -> {
						require(hex(a) >= 0 && hex(b) >= 0, part, at, why)
						grammar(after, part, why, allowed, at + 3)
					}
					_ -> problem(part, at, why)
				}
				False -> grammar(tail, part, why, allowed, at + 1)
			}
		}
	}
}

// Authority structure is split once; component validation stays in validate.
read_authority = (
	raw: List(Int)
	start: Int
) -> (Authority, List((UriPart, Int))) ^ UriError {
	(before_at, after_at, at) = cut(raw, 64)
	(userinfo, hostport, hstart) = at |> {
		True -> {
			(before_second, _, second) = cut(after_at, 64)
			require(
				second == False
				AuthorityPart
				start + list.length(before_at) + 1 + list.length(before_second)
				BadAuthority
			)
			(
				Some(text_bytes(before_at))
				after_at
				start + list.length(before_at) + 1
			)
		}
		False -> (None, raw, start)
	}
	// Structure selects the components; userinfo grammar precedes host grammar.
	fail.map_error(
		() { grammar_option(userinfo, UserInfoPart, BadAuthority, user_char) }
		(e: UriError) {
			UriError(part: e.part, offset: start + e.offset, problem: e.problem)
		}
	)
	(host, port, host_start, port_start) = hostport |> {
		[91, ..inside] -> {
			(h, after, closed) = cut(inside, 93)
			require(closed, HostPart, start + list.length(raw), BadHost)
			end = hstart + list.length(h) + 2
			p = after |> {
				[]         -> None
				[58, ..ps] -> Some(text_bytes(ps))
				_          -> problem(HostPart, end, BadHost)
			}
			(IpLiteral(text_bytes(h)), p, hstart + 1, end + 1)
		}
		_ -> {
			(h, p, colon) = cut(hostport, 58)
			fail.map_error(
				() { grammar(h, HostPart, BadHost, reg_char, 0) }
				(e: UriError) {
					UriError(
						part:    e.part
						offset:  hstart + e.offset
						problem: e.problem
					)
				}
			)
			(extra, _, again) = cut(p, 58)
			require(
				again == False
				HostPart
				hstart + list.length(h) + 1 + list.length(extra)
				BadHost
			)
			(
				RegName(text_bytes(h))
				optional_text(p, colon)
				hstart
				hstart + list.length(h) + 1
			)
		}
	}
	(
		Authority(userinfo: userinfo, host: host, port: port)
		[(UserInfoPart, start), (HostPart, host_start), (PortPart, port_start)]
	)
}

alpha = (x: Int) -> Bool { { x >= 65 && x <= 90 } || { x >= 97 && x <= 122 } }

digit = (x: Int) -> Bool { x >= 48 && x <= 57 }

unreserved = (x: Int) -> Bool {
	alpha(x) || digit(x) || list.contains([45, 46, 95, 126], x)
}

subdelim = (x: Int) -> Bool {
	list.contains([33, 36, 38, 39, 40, 41, 42, 43, 44, 59, 61], x)
}

reg_char = (x: Int) -> Bool { unreserved(x) || subdelim(x) || x == 37 }

user_char = (x: Int) -> Bool { reg_char(x) || x == 58 }

pchar = (x: Int) -> Bool { user_char(x) || x == 64 }

path_char = (x: Int) -> Bool { pchar(x) || x == 47 }

query_char = (x: Int) -> Bool { path_char(x) || x == 63 }

uri_char = (x: Int) -> Bool { query_char(x) || list.contains([35, 91, 93], x) }

no_colon = (x: Int) -> Bool { pchar(x) && x != 58 }

scheme_char = (x: Int) -> Bool {
	alpha(x) || digit(x) || list.contains([43, 45, 46], x)
}

valid_scheme = (xs: List(Int)) -> Bool {
	xs |> {
		[]          -> False
		[x, ..tail] -> alpha(x) && list.all(tail, scheme_char)
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

host_text = (h: Host) -> String {
	h |> {
		RegName(s)   -> s
		IpLiteral(s) -> s
	}
}

// Count IPv6 words, with an IPv4 tail worth two. Compression must omit at
// least one word. Empty sides are permitted only at the one double colon.
valid_ip = (s: String) -> Bool {
	bytes(s) |> {
		[118, ..tail] -> future(tail)
		[86, ..tail]  -> future(tail)
		_ -> {
			halves = string.split(s, "::")
			halves |> {
				[whole] -> groups(whole, True) == Some(8)
				[left, right] -> (groups(left, False), groups(right, True)) |> {
					(Some(a), Some(b)) -> a + b < 8
					_                  -> False
				}
				_ -> False
			}
		}
	}
}

future = (xs: List(Int)) -> Bool {
	(version, address, dot) = cut(xs, 46)
	dot &&
		version != [] &&
		list.all(version, (x) { hex(x) >= 0 }) &&
		address != [] &&
		list.all(address, (x) { unreserved(x) || subdelim(x) || x == 58 })
}

groups = (s: String, allow_ipv4: Bool) -> Option(Int) {
	s |> {
		"" -> Some(0)
		_ -> {
			go = (rest, count) {
				rest |> {
					[] -> Some(count)
					[part, ..tail] -> {
						xs = bytes(part)
						{
							allow_ipv4 && tail == [] && list.contains(xs, 46)
						} |> {
							True -> { ipv4(part) } |> {
								True  -> Some(count + 2)
								False -> None
							}
							False -> {
								xs != [] &&
									list.length(xs) <= 4 &&
									list.all(xs, (x) { hex(x) >= 0 })
							} |> {
								True  -> go(tail, count + 1)
								False -> None
							}
						}
					}
				}
			}
			go(string.split(s, ":"), 0)
		}
	}
}

ipv4 = (s: String) -> Bool {
	pieces = string.split(s, ".")
	list.length(pieces) == 4 && list.all(pieces, (part) {
		xs = bytes(part)
		no_zero = xs |> {
			[48, _, .._] -> False
			_            -> True
		}
		xs != [] &&
			list.length(xs) <= 3 &&
			no_zero &&
			list.all(xs, digit) &&
			list.fold(xs, 0, (n, x) { n * 10 + x - 48 }) <= 255
	})
}

bytes = (s: String) -> List(Int) { octets(<<s:utf8>>) }

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
	// Only original UTF-8 slices or ASCII byte lists reach this helper.
	bit_array.to_string(bit_array.concat(list.map(xs, (x) { <<x>> }))) |> {
		Ok(s)    -> s
		Error(_) -> panic("uri: invalid internal text")
	}
}

cut = (xs: List(Int), delimiter: Int) -> (List(Int), List(Int), Bool) {
	go = (rest, acc) {
		rest |> {
			[]                           -> (list.reverse(acc), [], False)
			[x, ..tail] ? x == delimiter -> (list.reverse(acc), tail, True)
			[x, ..tail]                  -> go(tail, [x, ..acc])
		}
	}
	go(xs, [])
}

optional_text = (xs: List(Int), present: Bool) -> Option(String) {
	present |> {
		True  -> Some(text_bytes(xs))
		False -> None
	}
}

delimited = (s: Option(String), before: String, after: String) -> String {
	s |> {
		None       -> ""
		Some(text) -> string.concat([before, text, after])
	}
}

// The output is a reversed stack of complete path segments, including each
// segment's leading slash. Removing the last segment therefore costs O(1).
remove_dots = (path: String) -> String {
	go = (rest: List(Int), out: List(String)) -> String {
		rest |> {
			[46, 46, 47, ..tail]     -> go(tail, out)
			[46, 47, ..tail]         -> go(tail, out)
			[47, 46, 47, ..tail]     -> go([47, ..tail], out)
			[47, 46]                 -> go([47], out)
			[47, 46, 46, 47, ..tail] -> go([47, ..tail], pop_segment(out))
			[47, 46, 46]             -> go([47], pop_segment(out))
			[46]                     -> go([], out)
			[46, 46]                 -> go([], out)
			[]                       -> string.concat(list.reverse(out))
			_ -> {
				(leading, remaining) = rest |> {
					[47, ..tail] -> ([47], tail)
					_            -> ([], rest)
				}
				(segment, tail, slash) = cut(remaining, 47)
				next = slash |> {
					True  -> [47, ..tail]
					False -> []
				}
				go(next, [text_bytes(list.append(leading, segment)), ..out])
			}
		}
	}
	go(bytes(path), [])
}

pop_segment = (out: List(String)) -> List(String) {
	out |> {
		[]          -> []
		[_, ..tail] -> tail
	}
}

merge_path = (base: Parts, relative: String) -> String {
	{ base.authority != None && base.path == "" } |> {
		True -> string.concat(["/", relative])
		False -> {
			go = (rest, prefix, segment) {
				rest |> {
					[] -> text_bytes(
						list.append(list.reverse(prefix), bytes(relative))
					)
					[47, ..tail] -> go(
						tail
						[47, ..list.append(segment, prefix)]
						[]
					)
					[x, ..tail] -> go(tail, prefix, [x, ..segment])
				}
			}
			go(bytes(base.path), [], [])
		}
	}
}

hex_digit = (n: Int) -> Int {
	{ n < 10 } |> {
		True  -> 48 + n
		False -> 55 + n
	}
}

encode_octets = (xs: List(Int), form: Bool) -> String {
	fragments = list.map(xs, (x) {
		{ form && x == 32 } |> {
			True -> <<43>>
			False -> unreserved(x) |> {
				True  -> <<x>>
				False -> <<37, hex_digit(x / 16), hex_digit(x % 16)>>
			}
		}
	})
	bit_array.to_string(bit_array.concat(fragments)) |> {
		Ok(s)    -> s
		Error(_) -> panic("uri: invalid encoded ASCII")
	}
}

decode_raw = (xs: List(Int), form: Bool) -> BitArray ^ CodecError {
	go = (
		rest: List(Int)
		at: Int
		acc: List(BitArray)
	) -> BitArray ^ CodecError {
		rest |> {
			[] -> bit_array.concat(list.reverse(acc))
			[x, ..tail] -> { x == 37 } |> {
				True -> tail |> {
					[a, b, ..after] -> { hex(a) >= 0 && hex(b) >= 0 } |> {
						True -> go(
							after
							at + 3
							[<<hex(a) * 16 + hex(b)>>, ..acc]
						)
						False -> fail.raise(BadEscape(offset: at))
					}
					_ -> fail.raise(BadEscape(offset: at))
				}
				False -> { x >= 128 } |> {
					True -> fail.raise(NonAscii(offset: at))
					False -> {
						value = { form && x == 43 } |> {
							True  -> 32
							False -> x
						}
						go(tail, at + 1, [<<value>>, ..acc])
					}
				}
			}
		}
	}
	go(xs, 0, [])
}

decode_text = (text: String, form: Bool) -> String ^ CodecError {
	bit_array.to_string(decode_raw(bytes(text), form)) |> {
		Ok(s)    -> s
		Error(_) -> fail.raise(InvalidUtf8)
	}
}
