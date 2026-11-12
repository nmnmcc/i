// spec: 4.5, 9.2
// expect: pass
// N <: First | (N - First) must not close an inferred record callback row.
First {
	First(run: () -> Nil)
}

Second {
	Second(run: () -> Nil)
}

operation = (second ! Second) { second.run() }

handle(A, E, N) = (body: () -> A ^ E ! N) -> A ^ E ! First | N - First {
	body()
}

wrapper = (p: (send: _)) { handle(() { p.send() }) }

main = () {
	! First = First(run: () { Nil })
	! Second = Second(run: () { Nil })
	wrapper((send: () { operation() }))
}
