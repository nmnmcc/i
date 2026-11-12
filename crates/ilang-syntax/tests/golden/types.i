T1 = A | B & C - D - E
T2 = { A | B } & C
T3 = A - { B - C }
T4 = { (A) -> B } | C
T5 = (A) <- (E) -> (B) -> C
T6 = (A) -> B | C
T7 = { (Int, String) }
T8 = Record
T9 = { (Int) }
T10 = m.T(_, A)
T11 = List(List(Int))
T12 = (A) -> (B) <- (Fail(E), N) -> C
T13 = (A) <- (N) -> (B) <- (Fail(E)) -> C
T14 = { A - B } - C
T15 = (() <- (Fail(E), N) -> A) <- (N) -> Result(A, E)
T16 = { A | B } | C
T17 = A & { (B) -> C } & D
T18 = (String, String) -> String
Opaque = Opaque
Pair(A, B) = (first = A, second = B)
Circle = (Float)
Rect = (w = Float, h = Float)
Dot = ()
Shape = Circle | Rect | Dot
Yield(A) = (emit = (A) -> Nil)
Gen(A) = Yield(A)

@external("m", "f")
f: <B>(Int, B) <- (Fail(E), N) -> Int | String
