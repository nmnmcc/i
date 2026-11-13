// spec: 4.4, 9.2
// expect: pass

// Lower bounds alone do not close A or B: both may still widen to a common
// supertype. Contrast the callbacks that supply upper bounds as well.
Read(A) = (read = () -> A)
read = <A>(x: A) <- (r: Read(A)) { r.read() }
both = <A, B>(x: A, y: B) { read(x) read(y) }
use = () { both(1, "s") }
