/// Writes a string to standard output (stdout).
///
/// If you want your output to be printed on its own line see `println`.
///
/// ## Example
///
/// ```i
/// assert io.print("Hi mum") == Nil
/// // Hi mum
/// ```
///
@external(wasm, "ilang:runtime/v1", "io_print")
pub fn print(string: String) -> Nil

/// Writes a string to standard error (stderr).
///
/// If you want your output to be printed on its own line see `println_error`.
///
/// ## Example
///
/// ```i
/// assert io.print_error("Hi pop") == Nil
/// // Hi pop
/// ```
///
@external(wasm, "ilang:runtime/v1", "io_print_error")
pub fn print_error(string: String) -> Nil

/// Writes a string to standard output (stdout), appending a newline to the end.
///
/// ## Example
///
/// ```i
/// assert io.println("Hi mum") == Nil
/// // Hi mum
/// ```
///
pub fn println(string: String) -> Nil {
  print(string <> "\n")
}

/// Writes a string to standard error (stderr), appending a newline to the end.
///
/// ## Example
///
/// ```i
/// assert io.println_error("Hi pop") == Nil
/// // Hi pop
/// ```
///
pub fn println_error(string: String) -> Nil {
  print_error(string <> "\n")
}
