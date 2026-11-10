// §6.10: the names of a binding with a direct value may be used inside
// the function bodies of that value.
(even, odd) = ((n) { n == 0 || odd(n - 1) }, (n) { n != 0 && even(n - 1) })

f = (n) { f(n - 1) }

// A direct value only reads names outside its function bodies (§2.5).
c = (1, d)
d = () { c }

wrapped = Wrap((n) { wrapped })
Wrap { Wrap(f: (Int) -> Wrap) }

local = () {
  fact = (n) {
    case n {
      0 -> 1
      _ -> n * fact(n - 1)
    }
  }
  grouped = { (n) { grouped(n) } }
  x = 1
  x = x + 1
  [_, _, __rest] = [x, x, x]
  _named = 1
  (a, b) = (fact(x), _named)
  <<len:u8, body:bytes(len)>> = <<1, 2>>
  (a, b, body)
}

// Type variables: `a` in the inner binding is the variable of `outer`.
outer = (x: a) -> a {
  inner: (a) -> a = (y) { y }
  inner(x)
}
