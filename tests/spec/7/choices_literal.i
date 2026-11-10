// spec: 4.8, 6.12
// expect: pass
f = { 0 | 1 -> 2 }
select = (x: Int | String) { x |> { n = 0 | n: String -> n } }
@test
choices = () {
    assert(f(0) == 2)
    assert(f(1) == 2)
    assert(select(0) == 0)
    assert(select("s") == "s")
}
