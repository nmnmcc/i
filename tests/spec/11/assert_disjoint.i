// spec: 6.9, 11.7
// expect: stop "assertion failed: assert"

// W520: the comparison is legal; Assert still reports failure at runtime.
main = () { assert(1 == 2) }
