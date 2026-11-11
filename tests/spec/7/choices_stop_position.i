// spec: 6.12, 7.3, 11.7
// expect: stop "assertion failed: pattern 0 | 1 did not match 2"
// at: spec/choices_stop_position:7:7
// at: spec/choices_stop_position:9:13

// §6.12: a fitted clause reports its first pattern, like a parameter.
f = { 0 | 1 -> 2 }

main = () { f(2)  Nil }
