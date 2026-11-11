// spec: 4.8, 6.12, 7.3
// expect: stop "panic: group choices"
pair = ("b", () { pair |> { ("a", _) | ("c", _) -> 1 } })

main = () {
    ! Assert = Assert((_) { panic("group choices") })
    pair.1()
}
