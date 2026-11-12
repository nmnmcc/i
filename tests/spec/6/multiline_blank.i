// spec: 1.6
// expect: pass
main = () {
    assert("""
      a
 
      b
      """ == "a\n\nb")
    Nil
}
