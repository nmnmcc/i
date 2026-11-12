// spec: 6.9
// expect: pass
main = () {
    x = "i"
    assert("hello \(x)!" == "hello i!")
    assert("\(x)" == x)
    assert("\(x)\(x)" == "ii")
    assert("" <> "a\(x)b" <> "!" == "aib!")
    Nil
}
