// spec: 10.2, B.5
// expect: compile-error E0321

// The program entry and a test have the same export name, so one function
// may not be both.
@main
@test
both = () { Nil }
