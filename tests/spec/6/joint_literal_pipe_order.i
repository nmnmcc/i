// spec: 6.2, 4.7
// expect: stop "panic: pipe value"
// Joint J3: the pipe value is evaluated before the other written arguments.
method = "GET"
port = 8080
take = (first : method, last : port) { (first, last) }

main = () {
	_ = panic("pipe value") |> take(panic("argument"))
	Nil
}
