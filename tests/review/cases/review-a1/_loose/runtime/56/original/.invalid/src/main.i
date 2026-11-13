// old-syntax-check: E0101,E0101,E0106
f = (n: Int) { case n { 0
| 1
if True
-> 42
_ -> 7 } }


main = () { assert(f(0) == 42 && f(1) == 42 && f(2) == 7) Nil }
