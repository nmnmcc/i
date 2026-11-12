// spec: 1.6
// expect: pass
main = () {
    x = "i"
    assert(#"hash \#(x) \#n"# == "hash i \n")
    assert(##"\#(x) \##(x) \#n\##n"## == "\\#(x) i \\#n\n")
    assert(###"\##n\###u{61}"### == "\\##na")
    Nil
}
