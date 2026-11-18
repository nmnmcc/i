// spec: 9.2
// expect: compile-error E0501
// Decision 2 (EF03/T4): Fail(Never) is a real service, even if unused.
requires = () <- (f : Fail(Never)) { () }
pure : () -> () = () { requires() }
main = () { () }
