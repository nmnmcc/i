// spec: 4.8, 9.2
// expect: compile-error E0515
raise = () <- (handler: Fail(String)) { handler.raise("wrong") }
// Rule B prevents N from hiding a second Fail beside the written Fail(Int).
f = <N>() <- (handler: Fail(Int), ..env: N) { raise() }
