// spec: 4.5, 9.2
// expect: pass
// N <: First | (N - First) must not close an inferred record callback row.
First = (run = () -> Nil)

Second = (run = () -> Nil)

operation = () <- (second: Second) { second.run() }

handle = <A, N>(body: () <- N -> A) -> A { body() }

wrapper = (p) { handle(() { p.send() }) }

main = () {
	wrapper((send = () { operation() })) <- (
		First(run = () { Nil }),
		Second(run = () { Nil }),
	)
}
