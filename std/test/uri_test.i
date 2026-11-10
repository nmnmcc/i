(
	std/uri
	std/list
	std/string
	std/bit_array
	(Some, None) = std/option
)

// U3 and §6.3: every successful route to Uri preserves raw spelling.
round_trip = (u: uri.Uri) -> Nil ! Assert {
	assert(uri.parse(uri.to_string(u)) == Ok(u))
	assert(uri.build(uri.parts(u)) == Ok(u))
	Nil
}

parse_good = (s: String) -> uri.Uri ! Assert {
	Ok(u) = uri.parse(s)
	assert(uri.to_string(u) == s)
	round_trip(u)
	u
}

@test
uri_raw_parts = () {
	xs = [
		""
		"?"
		"#"
		"?#"
		"a"
		"a/b:c"
		"a:b"
		"HTTP://User%3A@EXAMPLE.com:/a%2fb?x=+&x=%FF#f"
		"//"
		"//@:"
		"http://[::1]:/"
		"http://[v1.a:b]/"
		"/%FF"
		"//999.999.999.999:99999999999999999999999999999999999999/"
		"x:/a//b/../c"
		"mailto:a@example.com"
	]
	list.each(xs, (s) { parse_good(s) })
	u = parse_good("HTTP://u%3A@Host:/p%2F?q=#")
	assert(
		uri.parts(u) ==
			uri.Parts(
				Some("HTTP")
				Some(uri.Authority(Some("u%3A"), uri.RegName("Host"), Some("")))
				"/p%2F"
				Some("q=")
				Some("")
			)
	)
	assert(uri.has_scheme(u))
	assert(uri.has_scheme(parse_good("a/b")) == False)
	assert(
		uri.parts(parse_good("//h")).authority ==
			Some(uri.Authority(None, uri.RegName("h"), None))
	)
	assert(
		uri.parts(parse_good("//@h:")).authority ==
			Some(uri.Authority(Some(""), uri.RegName("h"), Some("")))
	)
	assert(
		uri.decode_component(uri.parts(parse_good("/%FF")).path) ==
			Error(uri.InvalidUtf8)
	)
	Nil
}

// U1/U8 selection and translation, including byte counts rather than codepoints.
@test
uri_parse_errors = () {
	cases = [
		("a b", uri.UriError(uri.WholePart, 1, uri.BadCharacter))
		("a%2", uri.UriError(uri.WholePart, 1, uri.BadPercent))
		("%GG ", uri.UriError(uri.WholePart, 0, uri.BadPercent))
		("é%GG", uri.UriError(uri.WholePart, 0, uri.BadCharacter))
		("/a\u{0}", uri.UriError(uri.WholePart, 2, uri.BadCharacter))
		("1:x", uri.UriError(uri.PathPart, 1, uri.BadPath))
		(":x", uri.UriError(uri.PathPart, 0, uri.BadPath))
		("http://[::1", uri.UriError(uri.HostPart, 11, uri.BadHost))
		("http://h:abc/", uri.UriError(uri.PortPart, 9, uri.BadPort))
		("http://h:1:2/", uri.UriError(uri.HostPart, 10, uri.BadHost))
		("//u@v@h", uri.UriError(uri.AuthorityPart, 5, uri.BadAuthority))
		("//[::1]x", uri.UriError(uri.HostPart, 7, uri.BadHost))
		("//[no]", uri.UriError(uri.HostPart, 3, uri.BadHost))
		("//a]", uri.UriError(uri.HostPart, 3, uri.BadHost))
		("a?b[c", uri.UriError(uri.QueryPart, 3, uri.BadCharacter))
		("a#b#c", uri.UriError(uri.FragmentPart, 3, uri.BadCharacter))
		("a[b", uri.UriError(uri.PathPart, 1, uri.BadPath))
		("//u[ser@h", uri.UriError(uri.UserInfoPart, 3, uri.BadAuthority))
		("//u[ser@h:1:2", uri.UriError(uri.UserInfoPart, 3, uri.BadAuthority))
		("//u[ser@[::", uri.UriError(uri.UserInfoPart, 3, uri.BadAuthority))
		("//h]:1:2", uri.UriError(uri.HostPart, 3, uri.BadHost))
	]
	list.each(cases, (pair) {
		(text, expected) = pair
		assert(uri.parse(text) == Error(expected))
	})
	Nil
}

@test
uri_build_errors = () {
	blank = uri.Parts(None, None, "", None, None)
	cases = [
		(
			uri.Parts(..blank, path: "a:b")
			uri.UriError(uri.PathPart, 1, uri.BadPath)
		)
		(
			uri.Parts(..blank, path: "//h")
			uri.UriError(uri.PathPart, 0, uri.BadPath)
		)
		(
			uri.Parts(
				..blank
				authority: Some(uri.Authority(None, uri.RegName("h"), None))
				path:      "x"
			)
			uri.UriError(uri.PathPart, 0, uri.BadPath)
		)
		(
			uri.Parts(..blank, scheme: Some(""))
			uri.UriError(uri.SchemePart, 0, uri.BadScheme)
		)
		(
			uri.Parts(..blank, scheme: Some("1x"))
			uri.UriError(uri.SchemePart, 0, uri.BadScheme)
		)
		(
			uri.Parts(..blank, scheme: Some("a:"))
			uri.UriError(uri.SchemePart, 1, uri.BadScheme)
		)
		(
			uri.Parts(..blank, scheme: Some("a%20"))
			uri.UriError(uri.SchemePart, 1, uri.BadScheme)
		)
		(
			uri.Parts(..blank, scheme: Some("é"))
			uri.UriError(uri.SchemePart, 0, uri.BadCharacter)
		)
		(
			uri.Parts(..blank, path: "x%")
			uri.UriError(uri.PathPart, 1, uri.BadPercent)
		)
		(
			uri.Parts(..blank, query: Some("#"))
			uri.UriError(uri.QueryPart, 0, uri.BadCharacter)
		)
		(
			uri.Parts(..blank, fragment: Some("["))
			uri.UriError(uri.FragmentPart, 0, uri.BadCharacter)
		)
		(
			uri.Parts(
				..blank
				authority: Some(
					uri.Authority(Some("a@"), uri.RegName("h"), None)
				)
			)
			uri.UriError(uri.UserInfoPart, 1, uri.BadAuthority)
		)
		(
			uri.Parts(
				..blank
				authority: Some(uri.Authority(None, uri.RegName("a:b"), None))
			)
			uri.UriError(uri.HostPart, 1, uri.BadHost)
		)
		(
			uri.Parts(
				..blank
				authority: Some(uri.Authority(None, uri.IpLiteral("::g"), None))
			)
			uri.UriError(uri.HostPart, 0, uri.BadHost)
		)
		(
			uri.Parts(
				..blank
				authority: Some(
					uri.Authority(None, uri.RegName("h"), Some("1x"))
				)
			)
			uri.UriError(uri.PortPart, 1, uri.BadPort)
		)
		// All lexical fields precede all field grammar and cross-field checks.
		(
			uri.Parts(..blank, scheme: Some("1"), path: "x%")
			uri.UriError(uri.PathPart, 1, uri.BadPercent)
		)
		(
			uri.Parts(..blank, path: "//h", query: Some(" "))
			uri.UriError(uri.QueryPart, 0, uri.BadCharacter)
		)
	]
	list.each(cases, (pair) {
		(p, expected) = pair
		assert(uri.build(p) == Error(expected))
	})
	Nil
}

// RFC 3986 §5.4 normal and abnormal examples; strict http:g behavior.
@test
uri_rfc_resolution = () {
	base = parse_good("http://a/b/c/d;p?q")
	cases = [
		("g:h", "g:h")
		("g", "http://a/b/c/g")
		("./g", "http://a/b/c/g")
		("g/", "http://a/b/c/g/")
		("/g", "http://a/g")
		("//g", "http://g")
		("?y", "http://a/b/c/d;p?y")
		("g?y", "http://a/b/c/g?y")
		("#s", "http://a/b/c/d;p?q#s")
		("g#s", "http://a/b/c/g#s")
		("g?y#s", "http://a/b/c/g?y#s")
		(";x", "http://a/b/c/;x")
		("g;x", "http://a/b/c/g;x")
		("g;x?y#s", "http://a/b/c/g;x?y#s")
		("", "http://a/b/c/d;p?q")
		(".", "http://a/b/c/")
		("./", "http://a/b/c/")
		("..", "http://a/b/")
		("../", "http://a/b/")
		("../g", "http://a/b/g")
		("../..", "http://a/")
		("../../", "http://a/")
		("../../g", "http://a/g")
		("../../../g", "http://a/g")
		("../../../../g", "http://a/g")
		("/./g", "http://a/g")
		("/../g", "http://a/g")
		("g.", "http://a/b/c/g.")
		(".g", "http://a/b/c/.g")
		("g..", "http://a/b/c/g..")
		("..g", "http://a/b/c/..g")
		("./../g", "http://a/b/g")
		("./g/.", "http://a/b/c/g/")
		("g/./h", "http://a/b/c/g/h")
		("g/../h", "http://a/b/c/h")
		("g;x=1/./y", "http://a/b/c/g;x=1/y")
		("g;x=1/../y", "http://a/b/c/y")
		("g?y/./x", "http://a/b/c/g?y/./x")
		("g?y/../x", "http://a/b/c/g?y/../x")
		("g#s/./x", "http://a/b/c/g#s/./x")
		("g#s/../x", "http://a/b/c/g#s/../x")
		("http:g", "http:g")
		("?", "http://a/b/c/d;p?")
		("g//h", "http://a/b/c/g//h")
		("%2e%2e/g", "http://a/b/c/%2e%2e/g")
		("%2F/g", "http://a/b/c/%2F/g")
	]
	list.each(cases, (pair) {
		(reference, expected) = pair
		Ok(u) = uri.resolve(base, parse_good(reference))
		assert(uri.to_string(u) == expected)
		round_trip(u)
	})
	// U5: no inherited base fragment; empty authority paths merge with a slash.
	Ok(u) = uri.resolve(parse_good("x://h#old"), parse_good("a"))
	assert(uri.to_string(u) == "x://h/a")
	round_trip(u)
	Ok(v) = uri.resolve(parse_good("x:a#old"), parse_good(""))
	assert(uri.to_string(v) == "x:a")
	round_trip(v)
	Nil
}

@test
uri_resolution_validation = () {
	base = parse_good("x:a")
	bad = Error(uri.UriError(uri.PathPart, 0, uri.BadPath))
	list.each(["/.//h", "/a/..//h", "/.//a:b", "x:/.//h"], (s) {
		assert(uri.resolve(base, parse_good(s)) == bad)
	})
	assert(
		uri.resolve(parse_good("a"), parse_good("/.//h")) ==
			Error(uri.UriError(uri.WholePart, 0, uri.RelativeBase))
	)
	assert(
		uri.resolve(parse_good("a"), parse_good("x:b")) ==
			Error(uri.UriError(uri.WholePart, 0, uri.RelativeBase))
	)
	Ok(u) = uri.resolve(parse_good("x://host/a"), parse_good("/.//h"))
	assert(uri.to_string(u) == "x://host//h")
	round_trip(u)
	Nil
}

@test
uri_ip_literals = () {
	valid = [
		"::"
		"::1"
		"1::"
		"1:2:3:4:5:6:7:8"
		"2001:db8::7"
		"::ffff:192.0.2.128"
		"1:2:3:4:5:6:192.0.2.1"
		"1:2:3:4:5::192.0.2.1"
		"v1.a:b"
		"VF.a!$&'()*+,;=:~_"
	]
	list.each(valid, (host) {
		parse_good(string.concat(["x://[", host, "]/"]))
	})
	invalid = [
		""
		":"
		":::1"
		"1:::2"
		"1::2::3"
		"1:2:3"
		"1:2:3:4:5:6:7:8:9"
		"1:2:3:4:5:6:7:8::"
		"1:2:3:4:5:6:7:"
		":1:2:3:4:5:6:7"
		"12345::"
		"::gg"
		"::ffff:256.1.2.3"
		"::ffff:01.2.3.4"
		"192.0.2.1::"
		"1:2:3:4:5:6::192.0.2.1"
		"fe80::1%25eth0"
		"v.a"
		"v1."
		"vG.a"
		"v1.a%20"
	]
	list.each(invalid, (host) {
		assert(
			uri.parse(string.concat(["x://[", host, "]"])) ==
				Error(uri.UriError(uri.HostPart, 5, uri.BadHost))
		)
	})
	Nil
}

@test
uri_codecs = () {
	assert(uri.encode_component("é /+%") == "%C3%A9%20%2F%2B%25")
	assert(uri.encode_component("AZaz09-._~") == "AZaz09-._~")
	assert(uri.decode_component("%252F+") == Ok("%2F+"))
	assert(uri.decode_component("%c3%a9%00") == Ok("é\u{0}"))
	assert(uri.decode_component("a b/?#[]") == Ok("a b/?#[]"))
	assert(uri.decode_bytes("%FF%00") == Ok(<<255, 0>>))
	assert(uri.encode_bytes(<<1:u1>>) == Error(uri.UnalignedBits))
	list.each(
		[
			"%FF"
			"%C0%AF"
			"%E0%80%80"
			"%F0%80%80%AF"
			"%C3"
			"%ED%A0%80"
			"%F4%90%80%80"
		]
		(s) { assert(uri.decode_component(s) == Error(uri.InvalidUtf8)) }
	)
	list.each([("%", 0), ("a%2", 1), ("abc%GG", 3)], (pair) {
		(s, at) = pair
		assert(uri.decode_bytes(s) == Error(uri.BadEscape(at)))
		assert(uri.decode_component(s) == Error(uri.BadEscape(at)))
	})
	assert(uri.decode_component("aé") == Error(uri.NonAscii(1)))
	all_bytes = bit_array.concat(list.map(list.range(0, 255), (n) { <<n>> }))
	Ok(encoded) = uri.encode_bytes(all_bytes)
	assert(uri.decode_bytes(encoded) == Ok(all_bytes))
	list.each(["", "hello", "é🙂\u{0} /%+", "-._~", "%252e"], (s) {
		assert(uri.decode_component(uri.encode_component(s)) == Ok(s))
	})
	Nil
}

@test
uri_forms = () {
	assert(
		uri.parse_query("a+b=c%2Bd&k&k=&a=2") ==
			Ok([("a b", "c+d"), ("k", ""), ("k", ""), ("a", "2")])
	)
	assert(uri.parse_query("&&x=1;2&&") == Ok([("x", "1;2")]))
	assert(uri.parse_query("x=1&y=%GG") == Error(uri.BadEscape(6)))
	assert(uri.parse_query("&&%GG=x") == Error(uri.BadEscape(2)))
	assert(uri.parse_query("x=1&é=2") == Error(uri.NonAscii(4)))
	assert(uri.parse_query("x=1&y=%FF") == Error(uri.InvalidUtf8))
	assert(
		uri.parse_query("a=b=c&=&a&a=") ==
			Ok([("a", "b=c"), ("", ""), ("a", ""), ("a", "")])
	)
	assert(
		uri.build_query([("b", "2"), ("a", " "), ("b", "+")]) == "b=2&a=+&b=%2B"
	)
	assert(uri.build_query([("", ""), ("a", ""), ("a", "+")]) == "=&a=&a=%2B")
	texts = ["", " ", "+", "é\u{0}", "a&=;?"]
	list.each(texts, (a) {
		list.each(texts, (b) {
			pairs = [(a, b), (b, a), (a, a)]
			assert(uri.parse_query(uri.build_query(pairs)) == Ok(pairs))
		})
	})
	Nil
}

// Generated valid Parts use U4's preservation law; short arbitrary ASCII
// strings use U1/U3, without treating self-produced expectations as grammar.
@test
uri_generated_round_trips = () {
	schemes = [None, Some("x"), Some("HTTP+1.-")]
	authorities = [
		None
		Some(uri.Authority(None, uri.RegName(""), None))
		Some(uri.Authority(Some("a%3A"), uri.RegName("h"), Some("")))
		Some(uri.Authority(None, uri.IpLiteral("::1"), Some("99999")))
	]
	list.each(schemes, (scheme) {
		list.each(authorities, (authority) {
			list.each(["", "/", "/a//b/../", "/%FF", "/a%2fb"], (path) {
				list.each([None, Some(""), Some("x=+&x=%FF")], (query) {
					list.each([None, Some(""), Some("a/?")], (fragment) {
						p = uri.Parts(scheme, authority, path, query, fragment)
						Ok(u) = uri.build(p)
						assert(uri.parts(u) == p)
						round_trip(u)
					})
				})
			})
		})
	})
	// Every ASCII byte alone and every pair of structural/lexical edge bytes.
	alphabet = [
		"a"
		"1"
		":"
		"/"
		"?"
		"#"
		"%"
		"@"
		"["
		"]"
		"+"
		"."
		" "
		"\u{0}"
	]
	check = (s) {
		uri.parse(s) |> {
			Ok(u) -> {
				assert(uri.to_string(u) == s)
				round_trip(u)
			}
			Error(_) -> Nil
		}
	}
	list.each(list.range(0, 127), (n) {
		Ok(cp) = string.utf_codepoint(n)
		check(string.from_codepoints([cp]))
	})
	list.each(alphabet, (a) {
		list.each(alphabet, (b) {
			list.each(alphabet, (c) { check(string.concat([a, b, c])) })
		})
	})
	Nil
}

@test
uri_long_inputs = () {
	long = string.repeat("a", 20000)
	text = string.concat([
		"x://"
		long
		":"
		string.repeat("9", 20000)
		"/"
		string.repeat("a/../", 10000)
		"z?"
		long
	])
	base = parse_good(text)
	Ok(u) = uri.resolve(base, parse_good("."))
	assert(
		uri.to_string(u) ==
			string.concat(["x://", long, ":", string.repeat("9", 20000), "/"])
	)
	round_trip(u)
	assert(uri.decode_component(uri.encode_component(long)) == Ok(long))
	pairs = [(long, long), ("", long)]
	assert(uri.parse_query(uri.build_query(pairs)) == Ok(pairs))
	Nil
}
