// spec: 4.4, 9.2
// expect: pass

// An open meet remains admissible until its callback types are instantiated.
Read(A) = (read = () -> A)
both = <A, B>(f: () <- (Read(A)) -> A, g: () <- (Read(B)) -> B) { f() g() }

// Decision 1: a method returning Never can stop, so Read(Never) is inhabited.
// Retain the required Read key; this type-only use need not be executed.
need_int = () <- (r: Read(Int)) { r.read() }
need_string = () <- (r: Read(String)) { r.read() }
use = () { both(need_int, need_string) }

main = () { () }
