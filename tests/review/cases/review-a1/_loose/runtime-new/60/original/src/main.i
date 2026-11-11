outer = (x: b) { inner(B) = (y: B, z: b) { z } inner(42, x) }


main = () { assert(outer("s") == "s") Nil }
