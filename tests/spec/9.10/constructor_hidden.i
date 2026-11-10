// spec: 9.10, 2.4
// expect: compile-error E0204

(
	i/random
)

// The public type is usable, but the constructor is private (§9.10).
forge = random.Generator
