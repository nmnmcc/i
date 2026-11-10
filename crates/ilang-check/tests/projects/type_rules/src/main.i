Box(A1) {
	Box(Int)
}

Free {
	Free(A1)
}

Twice(A1, A1) {
	Twice(A1)
}

Ali(A1) = Int

AliVar = List(A1)

A = B

B = List(A)

C = C

Rec {
	Cons(Int, Rec)
	End
}

D = Rec

Arity1 {
	Arity1(List)
}

Arity2 {
	Arity2(Int(Int))
}

Arity3 {
	Arity3(Result(Int))
}

Arity4 {
	Arity4(Box)
}

Arity5 {
	Arity5(Boundary(Int, Int, Int), Continuation(Int))
}

Missing {
	Missing(Nothing)
}

Clock {
	Clock(now: () -> Int)
}

U = Int | String

N = Never

K = Clock

envs = (c ! Clock, u ! U, n ! Never, k ! K, i ! Int) {
	! Clock = c
	! U = 1
	! N = 2
	1
}
