// spec: 10.2, 11.7
// expect: stop "panic: the entry stops"
// at: spec/test_stops:7:3

@main
main = () { panic("the entry stops") }
