S(A) { S(value: A) }
use(A) = (s ! S(A)) { s.value }
wrap(A) = (x: A) { use() }
