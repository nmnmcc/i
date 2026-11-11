/// Adds two numbers.
/// From the host.
@external("app:math/v1", "add")
add: (Int, Int) -> Int

@custom
main = () { Nil }

// not documentation
@deprecated("old")
@host("app:storage/v1", "async")
Storage { Storage(read: (key: String) -> String ^ String) }

@test
test_it = () { assert(True) }
