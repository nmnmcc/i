// spec: 13
// expect: pass
import (i/core)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

@test
booleans_and_numbers = () {
  check(show(core.not(True)), "False")
  check(show(core.not(False)), "True")
  check(show(core.to_float(3)), "3.0")
  check(show(core.to_float(-9007199254740993)), "-9.007199254740992e15")
  check(show(core.truncate(-.2.7)), "-2")
  check(show(core.truncate(2.7)), "2")
  check(show(core.truncate(1.0e20)), "100000000000000000000")
  check(show(core.floor(-.2.5)), "-3.0")
  check(show(core.floor(2.5)), "2.0")
  check(show(core.ceiling(2.1)), "3.0")
  check(show(core.ceiling(-.2.1)), "-2.0")
  check(show(core.sqrt(16.0)), "4.0")
  check(show(core.sqrt(2.0)), "1.4142135623730951")
  check(show(core.float_to_bits(1.0)), "4607182418800017408")
  check(show(core.float_to_bits(-.0.0)), "9223372036854775808")
  check(show(core.float_from_bits(4607182418800017408)), "1.0")
  check(show(core.float_from_bits(18446744073709551616 + 4607182418800017408)), "1.0")
}

@test
bits = () {
  check(show(core.bit_and(12, 10)), "8")
  check(show(core.bit_or(12, 10)), "14")
  check(show(core.bit_xor(12, 10)), "6")
  check(show(core.bit_and(-1, 255)), "255")
  check(show(core.bit_or(-256, 255)), "-1")
  check(show(core.bit_xor(-1, 100000000000000000000)), "-100000000000000000001")
  check(show(core.bit_not(5)), "-6")
  check(show(core.bit_not(-1)), "0")
  check(show(core.shift_left(1, 100)), "1267650600228229401496703205376")
  check(show(core.shift_left(8, -2)), "2")
  check(show(core.shift_right(-7, 1)), "-4")
  check(show(core.shift_right(7, 1)), "3")
  check(show(core.shift_right(1, -3)), "8")
}

@test
strings = () {
  check(core.concat(["a", "bc", "", "d"]), "abcd")
  check(core.concat([]), "")
  check(show(core.byte_size("héllo")), "6")
  check(show(core.byte_size("")), "0")
  check(show(core.byte_slice("héllo", 0, 1)), "Ok(\"h\")")
  check(show(core.byte_slice("héllo", 1, 2)), "Ok(\"é\")")
  check(show(core.byte_slice("héllo", 1, 1)), "Error(Nil)")
  check(show(core.byte_slice("héllo", 2, 1)), "Error(Nil)")
  check(show(core.byte_slice("abc", 2, 5)), "Error(Nil)")
  check(show(core.byte_slice("abc", -1, 1)), "Error(Nil)")
  check(show(core.byte_slice("abc", 3, 0)), "Ok(\"\")")
  check(show(core.codepoints("ab")), "[//utfcodepoint(a), //utfcodepoint(b)]")
  check(core.from_codepoints(core.codepoints("héllo 😀")), "héllo 😀")
  case core.codepoints("A") {
    [a] -> check(show(core.codepoint_to_int(a)), "65")
    _ -> panic("no")
  }
  check(show(core.int_to_codepoint(233)), "Ok(//utfcodepoint(é))")
  check(show(core.int_to_codepoint(55296)), "Error(Nil)")
  check(show(core.int_to_codepoint(1114112)), "Error(Nil)")
  check(show(core.int_to_codepoint(-1)), "Error(Nil)")
}

@test
bit_arrays = () {
  check(show(core.bit_size(<<1:u3>>)), "3")
  check(show(core.bit_size(<<>>)), "0")
  check(show(core.bit_array_to_string(<<104, 105>>)), "Ok(\"hi\")")
  check(show(core.bit_array_to_string(<<255>>)), "Error(Nil)")
  check(show(core.bit_array_to_string(<<104, 1:u1>>)), "Error(Nil)")
}

in_range = (h: Int) { h >= 0 && h < 4294967296 }

@test
inspect_and_hash = () {
  check(core.inspect(Ok([1])), "Ok([1])")
  check(show(core.hash([1, 2]) == core.hash([1, 2])), "True")
  check(show(core.hash(0.0) == core.hash(-.0.0)), "True")
  check(show(core.hash("é") == core.hash("é")), "True")
  check(show(in_range(core.hash(123456789012345678901234567890))), "True")
  check(show(in_range(core.hash((1, "a", <<1:u3>>)))), "True")
}

@test
as_values = () {
  fs = [core.not, core.not]
  case fs {
    [f, _] -> check(show(f(True)), "False")
    _ -> panic("no")
  }
  apply = (f, x) { f(x) }
  check(apply(core.inspect, 1), "1")
  check(show(apply(core.byte_size, "ab")), "2")
}
