(
	std/json
	std/property
	i/core
	std/list
	(Some, None) = std/option
)

@test
json_values = () {
	good = [
		"null"
		"true"
		"false"
		"0"
		"-1"
		"2.5"
		"1e2"
		"\"hello\""
		"[]"
		"{}"
		"[null,true,3]"
		"{\"a\":1}"
		"\"\\uD834\\uDD1E\""
		"\"é\""
	]
	list.each(good, (s) {
		Ok(v) = json.parse(s)
		assert(json.parse(json.print(v)) == Ok(v))
		assert(json.parse(json.pretty(v, 2)) == Ok(v))
	})
	bad = [
		""
		"nul"
		"true false"
		"01"
		"-01"
		"1."
		"1e"
		"[1,]"
		"{\"a\":1,}"
		"\"\\x\""
		"\"\\uD800\""
		"\"\\uDC00\""
		"\"\\uD800\\u0041\""
	]
	list.each(bad, (s) {
		assert(json.parse(s) |> {
			Error(_) -> True
			_        -> False
		})
	})
	assert(
		json.field(json.Object([("x", json.Integer(1))]), "x") ==
			Some(json.Integer(1))
	)
	assert(json.index(json.Array([]), 0) == None)
}

@test
json_conformance_cases = () {
	valid = [
		" null "
		"true"
		"false"
		"-0"
		"10"
		"-12"
		"0.0"
		"-0.1"
		"12.34"
		"1E+2"
		"1e-2"
		"[ ]"
		"{ }"
		"[1,2,3]"
		"[[],{}]"
		"{\"x\":true}"
		"{\"x\":1,\"x\":2}"
		"\"\""
		"\"/\""
		"\"\\/\""
		"\"\\\\\""
		"\"\\\"\""
		"\"\\b\\f\\n\\r\\t\""
		"\"\\u0000\""
		"\"\\u00e9\""
		"\"\\uD834\\uDD1E\""
		"\"你好\""
		"[true,false,null]"
		" { \"a\" : [1] } "
		"[\"a\",\"b\"]"
		"{\"a\":{},\"b\":[]}"
	]
	invalid = [
		""
		" "
		"Null"
		"TRUE"
		"undefined"
		"nan"
		"-"
		"+1"
		"00"
		"01"
		"-01"
		".1"
		"1."
		"1e"
		"1e+"
		"1e-"
		"["
		"]"
		"[1"
		"[1,]"
		"[,1]"
		"[1 2]"
		"{"
		"}"
		"{,}"
		"{\"a\"}"
		"{\"a\":}"
		"{\"a\":1,}"
		"{a:1}"
		"{\"a\" 1}"
		"\""
		"\"\\\""
		"\"\\x\""
		"\"\\u12\""
		"\"\\uZZZZ\""
		"\"\\uD800\""
		"\"\\uDC00\""
		"\"\\uD800\\u0041\""
		"true false"
		"nullx"
		"{}[]"
		"\"a\n\""
	]
	list.each(valid, (s) {
		Ok(v) = json.parse(s)
		assert(json.parse(json.print(v)) == Ok(v))
	})
	list.each(invalid, (s) {
		assert(json.parse(s) |> {
			Error(_) -> True
			_        -> False
		})
	})
}

@test
json_generated_strings = () {
	Ok(gen) = property.string(30)
	property.test(gen, property.Config(13, 100, 30, 0, 100), (s) {
		value = json.Object([("value", json.Text(s))])
		assert(json.parse(json.print(value)) == Ok(value))
		assert(json.parse(json.pretty(value, 2)) == Ok(value))
	}, core.inspect)
}
