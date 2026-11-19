//// The built-in names that are custom types or functions. Every
//// module may use them without an import, and other names may cover them.
//// This module may also be imported as `i/prelude`.
//// `Int`, `Float`, `String`, `List`, `BitArray`, `UtfCodepoint`, `Never`,
//// `Boundary`, and `Continuation` are built into the compiler.
(
	Bool(..)
	Result(..)
	AssertionFailure(..)
	Assert(..)
	Fail(..)
	panic
	assert
)

True = ()

False = ()
Bool = True | False

Ok(A) = (A)
Error(E) = (E)
Result(A, E) = Ok(A) | Error(E)

AssertionFailure = (message = String)

Assert = (fail = (AssertionFailure) -> Never)

/// The service that ends a computation with an error of type `E`.
Fail(E) = (raise = (E) -> Never)

/// Stops the program with `panic: ` and the message.
@external("ilang:prelude", "panic")
panic : (String) -> Never

/// Reports a failure to `Assert` when `c` is `False`.
assert = (c : Bool) <- (a : Assert) {
	c |>
		(True) { () }
		& (False) { a.fail(AssertionFailure(message = "assert")) }
}
