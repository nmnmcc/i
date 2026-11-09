// A second `@main` in a dependency is not an error either.
@main
main = () { Nil }

// The form of §2.2 is still checked.
@main
not_fn = 1
