// spec: 1.6
// expect: pass
main = () {
    name = "i"
    assert("""
      hello \
      \(name)!\nnext
      """ == "hello i!\nnext")
    Nil
}
