(
	i/fail
)

(
	call
)

@external("app:fallible/v1", "value")
value : (String) <- (Fail(String)) -> Int

call = (text : String) -> Int { value(text) }

main = () {
	assert(fail.catch(() { call("bad") }) == Error("from-host"))
	assert(call("good") == 42)
	()
}
