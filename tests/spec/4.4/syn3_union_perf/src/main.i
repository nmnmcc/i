C0 { C0 }
C1(A) { C1(A, E(A)) }
C2(A) { C2(A, E(A)) }
C3(A) { C3(A, E(A)) }
C4(A) { C4(A, E(A)) }
C5(A) { C5(A, E(A)) }
C6(A) { C6(A, E(A)) }
C7(A) { C7(A, E(A)) }
C8(A) { C8(A, E(A)) }
C9(A) { C9(A, E(A)) }
C10(A) { C10(A, E(A)) }
C11(A) { C11(A, E(A)) }
E(A) = C0 | C1(A) | C2(A) | C3(A) | C4(A) | C5(A) | C6(A) | C7(A) | C8(A) | C9(A) | C10(A) | C11(A)
f0 = (e) {
	e |> {
		C0 -> 0
		C1(a, r) -> a + f0(r)
		C2(a, r) -> a + f0(r)
		C3(a, r) -> a + f0(r)
		C4(a, r) -> a + f0(r)
		C5(a, r) -> a + f0(r)
		C6(a, r) -> a + f0(r)
		C7(a, r) -> a + f0(r)
		C8(a, r) -> a + f0(r)
		C9(a, r) -> a + f0(r)
		C10(a, r) -> a + f0(r)
		C11(a, r) -> a + f0(r)
	}
}
main = () { assert(f0(C1(1, C2(2, C0))) == 3) }
