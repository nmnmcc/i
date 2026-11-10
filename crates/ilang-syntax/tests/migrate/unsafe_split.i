f = (x) { x + 1 }
main = () { (f, n) = ((x: a) { f(x) }, length([1]))  f(n) }
