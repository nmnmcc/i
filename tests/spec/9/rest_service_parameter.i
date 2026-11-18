// spec: 6.8, 9.4
// expect: compile-error E0225
Db = ()
// The annotation on the rest must be the function's row parameter.
f = () <- (..env: Db) { () }
