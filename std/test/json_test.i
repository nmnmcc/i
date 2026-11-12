(
	testlib = (test) = std/test
	std/json
	std/property
	i/core
	std/list
	(Some, None) = std/option
)

test("json_values", () {
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
})

test("json_conformance_cases", () {
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
})

test("json_generated_strings", () {
	Ok(gen) = property.string(30)
	property.test(gen, property.Config(13, 100, 30, 0, 100), (s) {
		value = json.Object([("value", json.Text(s))])
		assert(json.parse(json.print(value)) == Ok(value))
		assert(json.parse(json.pretty(value, 2)) == Ok(value))
	}, core.inspect)
})

test("json_valid_0_null", () {
	Ok(v) = json.parse(" null ")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_1_true", () {
	Ok(v) = json.parse("true")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_2_false", () {
	Ok(v) = json.parse("false")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_3_0", () {
	Ok(v) = json.parse("-0")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_4_10", () {
	Ok(v) = json.parse("10")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_5_12", () {
	Ok(v) = json.parse("-12")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_6_0_0", () {
	Ok(v) = json.parse("0.0")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_7_0_1", () {
	Ok(v) = json.parse("-0.1")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_8_12_34", () {
	Ok(v) = json.parse("12.34")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_9_1e_2", () {
	Ok(v) = json.parse("1E+2")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_10_1e_2", () {
	Ok(v) = json.parse("1e-2")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_11_empty", () {
	Ok(v) = json.parse("[ ]")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_12_empty", () {
	Ok(v) = json.parse("{ }")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_13_1_2_3", () {
	Ok(v) = json.parse("[1,2,3]")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_14_empty", () {
	Ok(v) = json.parse("[[],{}]")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_15_x_true", () {
	Ok(v) = json.parse("{\"x\":true}")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_16_x_1_x_2", () {
	Ok(v) = json.parse("{\"x\":1,\"x\":2}")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_17_empty", () {
	Ok(v) = json.parse("\"\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_18_empty", () {
	Ok(v) = json.parse("\"/\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_19_empty", () {
	Ok(v) = json.parse("\"\\/\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_20_empty", () {
	Ok(v) = json.parse("\"\\\\\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_21_empty", () {
	Ok(v) = json.parse("\"\\\"\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_22_b_f_n_r_t", () {
	Ok(v) = json.parse("\"\\b\\f\\n\\r\\t\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_23_u0000", () {
	Ok(v) = json.parse("\"\\u0000\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_24_u00e9", () {
	Ok(v) = json.parse("\"\\u00e9\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_25_ud834_udd1e", () {
	Ok(v) = json.parse("\"\\uD834\\uDD1E\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_26_empty", () {
	Ok(v) = json.parse("\"你好\"")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_27_true_false_null", () {
	Ok(v) = json.parse("[true,false,null]")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_28_a_1", () {
	Ok(v) = json.parse(" { \"a\" : [1] } ")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_29_a_b", () {
	Ok(v) = json.parse("[\"a\",\"b\"]")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_valid_30_a_b", () {
	Ok(v) = json.parse("{\"a\":{},\"b\":[]}")
	assert(json.parse(json.print(v)) == Ok(v))
})

test("json_invalid_0_empty", () {
	assert(json.parse("") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_1_empty", () {
	assert(json.parse(" ") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_2_null", () {
	assert(json.parse("Null") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_3_true", () {
	assert(json.parse("TRUE") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_4_undefined", () {
	assert(json.parse("undefined") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_5_nan", () {
	assert(json.parse("nan") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_6_empty", () {
	assert(json.parse("-") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_7_1", () {
	assert(json.parse("+1") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_8_00", () {
	assert(json.parse("00") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_9_01", () {
	assert(json.parse("01") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_10_01", () {
	assert(json.parse("-01") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_11_1", () {
	assert(json.parse(".1") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_12_1", () {
	assert(json.parse("1.") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_13_1e", () {
	assert(json.parse("1e") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_14_1e", () {
	assert(json.parse("1e+") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_15_1e", () {
	assert(json.parse("1e-") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_16_empty", () {
	assert(json.parse("[") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_17_empty", () {
	assert(json.parse("]") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_18_1", () {
	assert(json.parse("[1") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_19_1", () {
	assert(json.parse("[1,]") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_20_1", () {
	assert(json.parse("[,1]") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_21_1_2", () {
	assert(json.parse("[1 2]") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_22_empty", () {
	assert(json.parse("{") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_23_empty", () {
	assert(json.parse("}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_24_empty", () {
	assert(json.parse("{,}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_25_a", () {
	assert(json.parse("{\"a\"}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_26_a", () {
	assert(json.parse("{\"a\":}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_27_a_1", () {
	assert(json.parse("{\"a\":1,}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_28_a_1", () {
	assert(json.parse("{a:1}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_29_a_1", () {
	assert(json.parse("{\"a\" 1}") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_30_empty", () {
	assert(json.parse("\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_31_empty", () {
	assert(json.parse("\"\\\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_32_x", () {
	assert(json.parse("\"\\x\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_33_u12", () {
	assert(json.parse("\"\\u12\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_34_uzzzz", () {
	assert(json.parse("\"\\uZZZZ\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_35_ud800", () {
	assert(json.parse("\"\\uD800\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_36_udc00", () {
	assert(json.parse("\"\\uDC00\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_37_ud800_u0041", () {
	assert(json.parse("\"\\uD800\\u0041\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_38_true_false", () {
	assert(json.parse("true false") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_39_nullx", () {
	assert(json.parse("nullx") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_40_empty", () {
	assert(json.parse("{}[]") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_invalid_41_a_n", () {
	assert(json.parse("\"a\n\"") |> {
		Error(_) -> True
		_        -> False
	})
})

test("json_float_overflow", () {
	assert(
		json.parse("1e400") == Error(json.JsonError(0, "number out of range"))
	)
})

test("json_arbitrary_integer", () {
	text = "12345678901234567890123456789012345678901234567890"
	Ok(value) = json.parse(text)
	assert(json.print(value) == text)
})
