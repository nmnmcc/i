S(A) { S }
use(A) = (s ! S(A)) { Nil }
wrap(A) = (x: A, f) { f(() { use() }) }
