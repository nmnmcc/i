// test-macro-migrate: reject
main = 1

(a, b) = ((), ())

c = (x) { x }

d = () <- (env : Assert) { Nil }

e = { () { Nil } }

@test
t = () { Nil }
