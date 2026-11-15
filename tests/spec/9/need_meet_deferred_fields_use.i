// spec: 4.4, 9.2
// expect: compile-error E0515

// An open meet remains admissible until its callback types are instantiated.
Read(A) = (value = A)
both = <A, B>(f: () <- (Read(A)) -> A, g: () <- (Read(B)) -> B) { f() g() }

// Results and needs together fix A = Int and B = String. A required value field cannot have type Never, so the empty service
// meet must report E0515 after freshening (Decision 1).
need_int = () <- (r: Read(Int)) { r.value }
need_string = () <- (r: Read(String)) { r.value }
use = () { both(need_int, need_string) }
