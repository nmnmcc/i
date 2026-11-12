@external("m", "f")
add: Int

@external("m", "f")
now: () -> Int

C(A, B) = (A, B)
T(A, B) = C(A, B)

U(A) = List(A)

C = (x = Int, y = Int)
V = C

W = Result(Int, String)

f = <A, E>(x: Result(A, E)) { x }

x: Result(Int, Int) = y
