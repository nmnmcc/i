Box(A1) = (Int)

Free = (A1)

Twice(A1, A1) = (A1)

Ali(A1) = Int

AliVar = List(A1)

A = B

B = List(A)

C = C

Cons = (Int, Rec)
End = ()
Rec = Cons | End

D = Rec

Arity1 = (List)

Arity2 = (Int(Int))

Arity3 = (Result(Int))

Arity4 = (Box)

Arity5 = (Boundary(Int, Int, Int), Continuation(Int))

Missing = (Nothing)

Clock = (now = () -> Int)

U = Int | String

N = Never

K = Clock

envs = () <- (c : Clock, u : U, n : Never, k : K, i : Int) { 1 <- (c, 1, 2) }
