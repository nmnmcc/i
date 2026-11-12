// spec: 4.8, 6.12, 7.3
// expect: stop "panic: outer choices"
f = (s: String) {
    s |> { "a" | "c" -> {
        ! Assert = Assert(fail: (_) { panic("body choices") })
        "x" = s
        1
    } }
}

main = () {
    ! Assert = Assert(fail: (_) { panic("outer choices") })
    f("b")
}
