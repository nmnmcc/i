// spec: 4.8, 6.12, 7.3
// expect: stop "panic: fitted choices"
f = (0 | 1) { 2 }

main = () { f(2) <- (Assert(fail = (_) { panic("fitted choices") })) }
