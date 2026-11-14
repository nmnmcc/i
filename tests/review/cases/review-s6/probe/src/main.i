Codec(d, e) = (decode = (String) -> d, encode = (e) -> String)
make = (d : (String) -> a, e : (b) -> String) { Codec(decode = d, encode = e) }
id = (x) { x }
size = id(make((s : String) -> Int { 1 }, (n : Int) -> String { "" }))
// old-syntax-check: E0101
