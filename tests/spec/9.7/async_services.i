// spec: 9.7, 10.2, B.4, B.5
// expect: pass
// output: double 1
// output: double 2
// output: echo 4
// output: double -1
// output: echo final
// output: double 5
// output: echo after
// output: 4 Error("negative") 10
(
	testlib = (test) = std/test
	i/control
	i/core
	i/fail
	i/resource
)

// An async host service: the program waits at each call, and goes on when
// the host answers through `ilang_resume:…`. Several calls are made one
// after the other, also through library code and inside a boundary of the
// program, and a final function waits too.
@host("test:async/v1", "async")
Later = (double = (Int) <- (Fail(String)) -> Int, echo = (String) -> String)

@host("test:io/v1")
Io = (print = (String) -> Nil)

twice = (n : Int) <- (later : Later) { later.double(later.double(n)) }

main = () <- (later : Later, io : Io) {
	a = twice(1)
	b = later.echo(core.inspect(a))
	c = fail.catch(() { later.double(-1) })
	d = resource.ensure(
		() { 5 },
		() {
			later.echo("final")
			Nil
		},
	)
	e = control.boundary((bd) {
		x = later.double(d)
		later.echo("after")
		x
	})
	io.print(b <> " " <> core.inspect(c) <> " " <> core.inspect(e))
}

// A test that may call an async function is async too.
test("async_test", () <- (later : Later) {
	later.double(21) |> (42) { Nil } | (n) { panic(core.inspect(n)) }
})
