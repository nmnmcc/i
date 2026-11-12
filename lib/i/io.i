//// The standard service (§9.7). A program that prints imports `Io` from
//// this module and takes it from the environment of its entry point.
(
	Io(..)
)

/// The host service that prints: `print` writes the text and a line feed
/// to the standard output. `i run`, `i test`, and the WASI target give it;
/// any other host gives it like any host service.
@host("i:io/v1")
Io = (print = (String) -> Nil)
