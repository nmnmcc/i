build = (
	c1 : Bool,
	c2 : Bool,
	c3 : Bool,
	c4 : Bool,
	c5 : Bool,
	c6 : Bool,
	c7 : Bool,
	c8 : Bool,
	c9 : Bool,
	c10 : Bool,
	c11 : Bool,
	c12 : Bool,
	c13 : Bool,
	c14 : Bool,
	c15 : Bool,
	c16 : Bool,
	c17 : Bool,
	c18 : Bool,
	c19 : Bool,
	c20 : Bool
) {
	r0 = ()
	r1 = c1 |> (True) { (..r0, a1 = 1) } | (False) { (..r0, b1 = 1) }
	r2 = c2 |> (True) { (..r1, a2 = 1) } | (False) { (..r1, b2 = 1) }
	r3 = c3 |> (True) { (..r2, a3 = 1) } | (False) { (..r2, b3 = 1) }
	r4 = c4 |> (True) { (..r3, a4 = 1) } | (False) { (..r3, b4 = 1) }
	r5 = c5 |> (True) { (..r4, a5 = 1) } | (False) { (..r4, b5 = 1) }
	r6 = c6 |> (True) { (..r5, a6 = 1) } | (False) { (..r5, b6 = 1) }
	r7 = c7 |> (True) { (..r6, a7 = 1) } | (False) { (..r6, b7 = 1) }
	r8 = c8 |> (True) { (..r7, a8 = 1) } | (False) { (..r7, b8 = 1) }
	r9 = c9 |> (True) { (..r8, a9 = 1) } | (False) { (..r8, b9 = 1) }
	r10 = c10 |> (True) { (..r9, a10 = 1) } | (False) { (..r9, b10 = 1) }
	r11 = c11 |> (True) { (..r10, a11 = 1) } | (False) { (..r10, b11 = 1) }
	r12 = c12 |> (True) { (..r11, a12 = 1) } | (False) { (..r11, b12 = 1) }
	r13 = c13 |> (True) { (..r12, a13 = 1) } | (False) { (..r12, b13 = 1) }
	r14 = c14 |> (True) { (..r13, a14 = 1) } | (False) { (..r13, b14 = 1) }
	r15 = c15 |> (True) { (..r14, a15 = 1) } | (False) { (..r14, b15 = 1) }
	r16 = c16 |> (True) { (..r15, a16 = 1) } | (False) { (..r15, b16 = 1) }
	r17 = c17 |> (True) { (..r16, a17 = 1) } | (False) { (..r16, b17 = 1) }
	r18 = c18 |> (True) { (..r17, a18 = 1) } | (False) { (..r17, b18 = 1) }
	r19 = c19 |> (True) { (..r18, a19 = 1) } | (False) { (..r18, b19 = 1) }
	r20 = c20 |> (True) { (..r19, a20 = 1) } | (False) { (..r19, b20 = 1) }
	r20
}

Fan(A) = { (a1 = A) | (b1 = Int) } & { (a2 = A) | (b2 = Int) } & { (a3 = A)
		| (b3 = Int) } & { (a4 = A) | (b4 = Int) } & { (a5 = A)
		| (b5 = Int) } & { (a6 = A) | (b6 = Int) } & { (a7 = A)
		| (b7 = Int) } & { (a8 = A) | (b8 = Int) } & { (a9 = A)
		| (b9 = Int) } & { (a10 = A) | (b10 = Int) } & { (a11 = A)
		| (b11 = Int) } & { (a12 = A) | (b12 = Int) } & { (a13 = A)
		| (b13 = Int) } & { (a14 = A) | (b14 = Int) } & { (a15 = A)
		| (b15 = Int) } & { (a16 = A) | (b16 = Int) } & { (a17 = A)
		| (b17 = Int) } & { (a18 = A) | (b18 = Int) } & { (a19 = A)
		| (b19 = Int) } & { (a20 = A) | (b20 = Int) }
Closed = Fan(Never)

closed : Closed = (
	b1 = 1,
	b2 = 1,
	b3 = 1,
	b4 = 1,
	b5 = 1,
	b6 = 1,
	b7 = 1,
	b8 = 1,
	b9 = 1,
	b10 = 1,
	b11 = 1,
	b12 = 1,
	b13 = 1,
	b14 = 1,
	b15 = 1,
	b16 = 1,
	b17 = 1,
	b18 = 1,
	b19 = 1,
	b20 = 1,
)
