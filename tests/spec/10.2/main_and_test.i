// spec: 10.2, B.5
// expect: compile-error E0305

// This function is not the default entry, and tests belong under test/.
@main
@test
both = () { Nil }
