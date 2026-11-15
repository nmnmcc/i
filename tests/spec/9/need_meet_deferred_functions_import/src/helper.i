(Read(..), both)

Read(A) = (value = A)
both = <A, B>(f: () <- (Read(A)) -> A, g: () <- (Read(B)) -> B) { f() g() }
