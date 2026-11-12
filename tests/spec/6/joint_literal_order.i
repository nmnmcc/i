// spec: 6.2, 4.7
// expect: stop "panic: first written"
// Joint J3: completed value types do not change labelled argument order.
method = "GET"
port = 8080
take = (first: method, last: port) { (first, last) }

main = () {
    _ = { _syn3_0 = panic("first written")  take(panic("second written"), _syn3_0) }
    Nil
}
