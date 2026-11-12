//// The built-in names of §12 that are custom types or functions. Every
//// module may use them without an import, and other names may cover them.
//// This module may also be imported as `i/prelude`.
//// `Int`, `Float`, `String`, `List`, `BitArray`, `UtfCodepoint`, `Never`,
//// `Boundary`, and `Continuation` are built into the compiler.

(
	Bool(..)
	Nil(..)
	Result(..)
	AssertionFailure(..)
	Assert(..)
	panic
	assert
)

Bool {
	True
	False
}

Nil {
	Nil
}

Result(A, E) {
	Ok(A)
	Error(E)
}

AssertionFailure {
	AssertionFailure(message: String)
}

Assert {
	Assert(fail: (AssertionFailure) -> Never)
}

/// Stops the program with `panic: ` and the message (§11.7).
@external("ilang:prelude", "panic")
panic: (String) -> Never

/// Reports a failure to `Assert` when `c` is `False` (§9.6).
/// type: (c: Bool) -> Nil ! Assert
assert = (c: Bool, a ! Assert) {
	c |> {
		True  -> Nil
		False -> a.fail(AssertionFailure(message: "assert"))
	}
}
