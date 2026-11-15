(m = probe/macros)
main = () {
    assert(m.id(value = 42) == 42)
    assert(m.select(right = "ignored", left = 42) == 42)
}
