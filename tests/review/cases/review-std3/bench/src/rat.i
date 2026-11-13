(i/core)
(R(..), make, add, sub, mul, div, atan_terms, sin_terms)
R { R(n: Int, d: Int) }
abs = (a: Int) -> Int { { a < 0 } |> { True -> -a  False -> a } }
gcd = (a: Int, b: Int) -> Int { b |> { 0 -> a  _ -> gcd(b, a % b) } }
make = (n: Int, d: Int) -> R {
  g = gcd(abs(n), d)
  R(n: n / g, d: d / g)
}
add = (a: R, b: R) -> R {
  g = gcd(a.d,b.d)
  n = a.n * { b.d / g } + b.n * { a.d / g }
  h = gcd(abs(n),g)
  R(n: n / h, d: { a.d / g } * { b.d / h })
}
sub = (a: R, b: R) -> R { add(a,R(n: -b.n,d: b.d)) }
mul = (a: R, b: R) -> R {
  g = gcd(abs(a.n),b.d)
  h = gcd(abs(b.n),a.d)
  R(n: { a.n / g } * { b.n / h }, d: { a.d / h } * { b.d / g })
}
div = (a: R, b: R) -> R {
  { b.n < 0 } |> {
    True -> mul(a,R(n: -b.d,d: -b.n))
    False -> mul(a,R(n: b.d,d: b.n))
  }
}
atan_terms = (z: R, count: Int) -> R {
  square = mul(z,z)
  go = (k: Int, term: R, sum: R) -> R {
    { k == count } |> {
      True -> sum
      False -> go(k+1, mul(term,R(n: -square.n,d: square.d)), add(sum,div(term,R(n: 2*k+1,d: 1))))
    }
  }
  go(0,z,R(n: 0,d: 1))
}
sin_terms = (z: R, count: Int) -> R {
  square = mul(z,z)
  go = (k: Int, term: R, sum: R) -> R {
    { k == count } |> {
      True -> sum
      False -> {
        next = div(mul(term,R(n: -square.n,d: square.d)),R(n: {2*k+2}*{2*k+3},d: 1))
        go(k+1,next,add(sum,term))
      }
    }
  }
  go(0,z,R(n: 0,d: 1))
}
