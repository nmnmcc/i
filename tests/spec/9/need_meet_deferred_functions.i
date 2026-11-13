// spec: 4.4, 9.2
// expect: pass

// An open meet remains admissible until its callback types are instantiated.
Read(A) = (read = () -> A)
both = <A, B>(f: () <- (Read(A)) -> A, g: () <- (Read(B)) -> B) { f() g() }

main = () { Nil }
