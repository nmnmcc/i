// spec: 9.7, 9.2
// expect: pass

// i code may give a host service with an environment statement, to put a
// fake in its place in a test (§9.7). Then the test does not need it from
// the host, and does not wait.
(
	testlib = (test) = std/test
)

@host("test:kv/v1")
Kv {
	Kv(get: (String) -> String ^ String)
}

@host("test:async/v1", "async")
Later {
	Later(echo: (String) -> String)
}

greeting = (kv ! Kv, later ! Later) { later.echo("hello " <> kv.get("name")) }

test("with_fakes", () {
	! Kv = Kv(get: (key) { "fake " <> key })
	! Later = Later(echo: (text) { text <> "!" })
	greeting() |> {
		"hello fake name!" -> Nil
		other              -> panic(other)
	}
})

test("with_the_host", (later ! Later) {
	! Kv = Kv(get: (key) { key })
	greeting() |> {
		"hello name" -> Nil
		other        -> panic(other)
	}
})
