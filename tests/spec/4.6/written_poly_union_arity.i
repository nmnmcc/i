// spec: 4.6, 4.7
// expect: compile-error E0501
// Chain-types probe 15: a unary call must satisfy both possible operands.
f = <A>(x : A) -> A { x } & <B>(x : B, y : B) -> B { _ = x
    y }
U = { <A>(A) -> A } | { <B>(B, B) -> B }
u : U = f
main = () { u(1) }
