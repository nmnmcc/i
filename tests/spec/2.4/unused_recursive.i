// spec: 2.4.1; references only from unused bindings do not count.
// expect: warning W0504

even = (n: Int) { n == 0 || odd(n - 1) }

odd = (n: Int) { n != 0 && even(n - 1) }

helper = () { assert(True) }

@test
check = () { helper() }

_quiet = 1
