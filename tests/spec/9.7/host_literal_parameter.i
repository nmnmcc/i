// spec: 9.7, 10.3
// expect: pass
// output: b

@host("test:io/v1")
Io { Io(print: (text: "a" | "b") -> Nil) }
@main
main = (io ! Io) { io.print("b") }
