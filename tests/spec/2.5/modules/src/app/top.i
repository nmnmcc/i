(
	base = (Green) = app/app/base
)

(
	answer
	colors
)

// A module comes after the modules it imports (§2.5).
answer = base.start + 2

colors = [base.Red, base.Blue(level: 0), Green]
