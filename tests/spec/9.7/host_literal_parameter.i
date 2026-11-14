// spec: 9.7, 10.3
// expect: pass
// output: b
@host("test:io/v1")
Io = (print = ("a" | "b") -> Nil)

main = () <- (io : Io) { io.print("b") }
