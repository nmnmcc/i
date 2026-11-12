// spec: 1.6
// expect: pass
main = () {
    x = "i"
    assert(#"""
      raw \n \(x)
        \#(x)\#n
      """# == "raw \\n \\(x)\n  i\n")
    assert(##"""
      a\##
      b
      """## == "ab")
    Nil
}
