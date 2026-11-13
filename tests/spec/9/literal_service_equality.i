// spec: 4.5, 9.2
// expect: compile-error E0515

// §4.5 rule 4: Counter(Int) cannot provide Counter(0), since Int is not a subtype of 0.
Counter(A) { Counter(get: () -> A) }
read = (c ! Counter(0)) { c.get() }
f = () {
	! Counter(Int) = Counter(get: () { 1 })
	read()
}
