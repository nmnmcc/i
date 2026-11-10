// spec: 11.8
// expect: pass
import (i/core)

check = (got: String, want: String) {
  case got == want {
    True -> Nil
    False -> panic(got <> " is not " <> want)
  }
}

show = (x) { core.inspect(x) }

User { User(name: String, age: Int) }
Unit { Unit }

@test
ints = () {
  check(show(0), "0")
  check(show(42), "42")
  check(show(-42), "-42")
  check(show(1073741823), "1073741823")
  check(show(-1073741824), "-1073741824")
  check(show(123456789012345678901234567890), "123456789012345678901234567890")
  check(show(-123456789012345678901234567890), "-123456789012345678901234567890")
}

@test
floats = () {
  check(show(0.0), "0.0")
  check(show(-.0.0), "-0.0")
  check(show(1.0), "1.0")
  check(show(-.1.0), "-1.0")
  check(show(123.0), "123.0")
  check(show(12.67), "12.67")
  check(show(0.0123), "0.0123")
  check(show(0.1), "0.1")
  check(show(100.0), "100.0")
  check(show(1000.0), "1.0e3")
  check(show(0.0001), "0.0001")
  check(show(0.00001), "1.0e-5")
  check(show(3.0e26), "3.0e26")
  check(show(1.0e-7), "1.0e-7")
  check(show(1.5e300), "1.5e300")
  check(show(123456789012.0), "123456789012.0")
  check(show(9007199254740992.0), "9.007199254740992e15")
  check(show(5.0e-324), "5.0e-324")
  check(show(1.7976931348623157e308), "1.7976931348623157e308")
  check(show(0.30000000000000004), "0.30000000000000004")
}

@test
strings = () {
  check(show(""), "\"\"")
  check(show("abc"), "\"abc\"")
  check(show("a\"b\\c"), "\"a\\\"b\\\\c\"")
  check(show("\n\r\t\f"), "\"\\n\\r\\t\\f\"")
  check(show("\u{0}\u{1F}\u{7F}\u{80}\u{9F}"), "\"\\u{0000}\\u{001F}\\u{007F}\\u{0080}\\u{009F}\"")
  check(show("é\u{A0}😀"), "\"é\u{A0}😀\"")
}

@test
collections = () {
  check(show([]), "[]")
  check(show([1]), "[1]")
  check(show([1, 2, 3]), "[1, 2, 3]")
  check(show(()), "()")
  check(show((1)), "(1)")
  check(show((1, "a", [2.0])), "(1, \"a\", [2.0])")
}

@test
custom = () {
  check(show(True), "True")
  check(show(False), "False")
  check(show(Nil), "Nil")
  check(show(Ok(1)), "Ok(1)")
  check(show(Error("e")), "Error(\"e\")")
  check(show(User("ann", 3)), "User(\"ann\", 3)")
  check(show(Unit), "Unit")
  check(show(Ok(Error(Ok(Nil)))), "Ok(Error(Ok(Nil)))")
}

@test
bit_arrays = () {
  check(show(<<>>), "<<>>")
  check(show(<<82, 2:u2>>), "<<82, 2:u2>>")
  check(show(<<1:u1>>), "<<1:u1>>")
  check(show(<<0, 255>>), "<<0, 255>>")
}

@test
codepoints = () {
  check(show(core.codepoints("aé")), "[//utfcodepoint(a), //utfcodepoint(é)]")
}

wide = (p0, p1, p2, p3, p4, p5, p6, p7, p8, p9, p10, p11, p12, p13, p14, p15, p16, p17, p18, p19, p20, p21, p22, p23, p24, p25, p26, p27) { p0 }

@test
functions = () {
  check(show(() { 1 }), "//fn() { ... }")
  check(show((a, b) { a }), "//fn(a, b) { ... }")
  check(show(show), "//fn(a) { ... }")
  check(show(wide), "//fn(a, b, c, d, e, f, g, h, i, j, k, l, m, n, o, p, q, r, s, t, u, v, w, x, y, z, a1, b1) { ... }")
  check(show(Ok), "//fn(a) { ... }")
  check(show(User), "//fn(a, b) { ... }")
  check(show(core.bit_and), "//fn(a, b) { ... }")
  check(show(panic), "//fn(a) { ... }")
  check(show([show]), "[//fn(a) { ... }]")
}
