// spec: 4.6, 4.7
// expect: pass
// Chain-types probe 24: alpha-renamed universal operands coalesce.
U = { <A>(A) -> A } | { <B>(B) -> B }
id : U = <C>(x : C) -> C { x }
main = () {
    assert(id(1) == 1)
    assert(id("s") == "s")
}
