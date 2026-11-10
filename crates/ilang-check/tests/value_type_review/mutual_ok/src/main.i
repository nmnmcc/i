even = (n: Int) { n == 0 || odd(n - 1) }
odd = (n: Int) { n != 0 && even(n - 1) }
