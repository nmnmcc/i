// spec: 4.4, 9.2
// expect: compile-error E0515

// An open meet remains admissible until its callback types are instantiated.
Read(A) = (read = () -> A)
both = <A, B>(f: () <- (Read(A)) -> A, g: () <- (Read(B)) -> B) { f() g() }

// Results and needs together fix A = Int and B = String. Neither operand
// was Never, so their empty service meet must report E0515 after freshening.
need_int = () <- (r: Read(Int)) { r.read() }
need_string = () <- (r: Read(String)) { r.read() }
use = () { both(need_int, need_string) }
