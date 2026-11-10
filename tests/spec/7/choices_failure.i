// spec: 4.8, 6.12, 7.3
// expect: stop "panic: fitted choices"
f = { 0 | 1 -> 2 }
@main
main = () {
    ! Assert = Assert((_) { panic("fitted choices") })
    f(2)
}
