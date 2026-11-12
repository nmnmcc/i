f = (x, _ = x = label, label: Int = x, (a, b) = x, (a, b), _) <- (
	clock: Clock,
	s: m.State(Int),
) { x }
g = (x, y: Int) <- (c: Clock) { x }
