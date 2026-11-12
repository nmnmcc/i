// spec: 4.5, 9.2
// expect: compile-error E0501

// §5.1 rule 1: arguments of services are compared with equal value sets.
Counter(A) = (get = () -> A)
read = () <- (c: Counter(0)) { c.get() }
f = () { read() <- (Counter(get = () { 1 })) }
