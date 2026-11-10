// §4.4: in a written type, a union with two or more type variables only in
// an output place, and an intersection with a type variable only in an
// input place.
union_in = (v: a | b) { v }
inter_out = (v: Int) -> a & Int { v }
inter_both = (x: a, y: b) -> a & b { x }
inter_param_of_param = (f: (a & Int) -> Int) { 1 }
union_in_alias = (v: Either(a, b)) { v }
Either(a, b) = a | b
// Allowed.
inter_in = (x: a & Int) -> a { x }
union_param_of_param = (f: (a | b) -> Int) { 1 }
union_out: (Int) -> a | b = (x) { panic("") }
Wrap(a, b) { Wrap(value: a | b) }
