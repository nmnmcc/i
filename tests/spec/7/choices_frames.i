// spec: 4.8, 6.12, 7.3
// expect: stop "panic: inner choices"
f = (s: String) -> Int {
    (a, b) = (1, 2)
    ! Assert = Assert((_) { panic("inner choices") })
    s |> { "a" | "c" -> a }
}

main = () { f("b") }
