// spec: 9.7, 9.3, 10.2, 11.7
// expect: stop "error: "negative""
// at: spec/main:15:1
// output: double 2
// output: double -1

// An error of an async host function that comes to the program entry stops
// it, after the program waited for the host.
@host("test:async/v1", "async")
Later = (double = (Int) <- (Fail(String)) -> Int)

main = () <- (later: Later) {
	x = later.double(2)
	later.double(x - 5)
	Nil
}
