S0(N) = (f = () <- N -> Int, m = (N) -> Nil)
u0 = <N>() <- (s: S0(N)) {
	S0(f = f, ..) = s
	f()
}
S1(N) = (f = () <- N -> Int, m = (N) -> Nil)
u1 = <N>() <- (s: S1(N)) {
	S1(f = f, ..) = s
	f()
}
S2(N) = (f = () <- N -> Int, m = (N) -> Nil)
u2 = <N>() <- (s: S2(N)) {
	S2(f = f, ..) = s
	f()
}
S3(N) = (f = () <- N -> Int, m = (N) -> Nil)
u3 = <N>() <- (s: S3(N)) {
	S3(f = f, ..) = s
	f()
}
S4(N) = (f = () <- N -> Int, m = (N) -> Nil)
u4 = <N>() <- (s: S4(N)) {
	S4(f = f, ..) = s
	f()
}
S5(N) = (f = () <- N -> Int, m = (N) -> Nil)
u5 = <N>() <- (s: S5(N)) {
	S5(f = f, ..) = s
	f()
}
S6(N) = (f = () <- N -> Int, m = (N) -> Nil)
u6 = <N>() <- (s: S6(N)) {
	S6(f = f, ..) = s
	f()
}
S7(N) = (f = () <- N -> Int, m = (N) -> Nil)
u7 = <N>() <- (s: S7(N)) {
	S7(f = f, ..) = s
	f()
}
S8(N) = (f = () <- N -> Int, m = (N) -> Nil)
u8 = <N>() <- (s: S8(N)) {
	S8(f = f, ..) = s
	f()
}
S9(N) = (f = () <- N -> Int, m = (N) -> Nil)
u9 = <N>() <- (s: S9(N)) {
	S9(f = f, ..) = s
	f()
}
S10(N) = (f = () <- N -> Int, m = (N) -> Nil)
u10 = <N>() <- (s: S10(N)) {
	S10(f = f, ..) = s
	f()
}
S11(N) = (f = () <- N -> Int, m = (N) -> Nil)
u11 = <N>() <- (s: S11(N)) {
	S11(f = f, ..) = s
	f()
}
S12(N) = (f = () <- N -> Int, m = (N) -> Nil)
u12 = <N>() <- (s: S12(N)) {
	S12(f = f, ..) = s
	f()
}
S13(N) = (f = () <- N -> Int, m = (N) -> Nil)
u13 = <N>() <- (s: S13(N)) {
	S13(f = f, ..) = s
	f()
}
S14(N) = (f = () <- N -> Int, m = (N) -> Nil)
u14 = <N>() <- (s: S14(N)) {
	S14(f = f, ..) = s
	f()
}
S15(N) = (f = () <- N -> Int, m = (N) -> Nil)
u15 = <N>() <- (s: S15(N)) {
	S15(f = f, ..) = s
	f()
}
S16(N) = (f = () <- N -> Int, m = (N) -> Nil)
u16 = <N>() <- (s: S16(N)) {
	S16(f = f, ..) = s
	f()
}
S17(N) = (f = () <- N -> Int, m = (N) -> Nil)
u17 = <N>() <- (s: S17(N)) {
	S17(f = f, ..) = s
	f()
}
S18(N) = (f = () <- N -> Int, m = (N) -> Nil)
u18 = <N>() <- (s: S18(N)) {
	S18(f = f, ..) = s
	f()
}
S19(N) = (f = () <- N -> Int, m = (N) -> Nil)
u19 = <N>() <- (s: S19(N)) {
	S19(f = f, ..) = s
	f()
}
all = () -> Int {
	u0() +
		u1() +
		u2() +
		u3() +
		u4() +
		u5() +
		u6() +
		u7() +
		u8() +
		u9() +
		u10() +
		u11() +
		u12() +
		u13() +
		u14() +
		u15() +
		u16() +
		u17() +
		u18() +
		u19()
}
g0 = () { all() }
g1 = () { all() + g0() }
g2 = () { all() + g1() }
g3 = () { all() + g2() }
g4 = () { all() + g3() }
g5 = () { all() + g4() }
g6 = () { all() + g5() }
g7 = () { all() + g6() }
g8 = () { all() + g7() }
g9 = () { all() + g8() }
g10 = () { all() + g9() }
g11 = () { all() + g10() }
g12 = () { all() + g11() }
g13 = () { all() + g12() }
g14 = () { all() + g13() }
g15 = () { all() + g14() }
g16 = () { all() + g15() }
g17 = () { all() + g16() }
g18 = () { all() + g17() }
g19 = () { all() + g18() }
g20 = () { all() + g19() }
g21 = () { all() + g20() }
g22 = () { all() + g21() }
g23 = () { all() + g22() }
g24 = () { all() + g23() }
g25 = () { all() + g24() }
g26 = () { all() + g25() }
g27 = () { all() + g26() }
g28 = () { all() + g27() }
g29 = () { all() + g28() }
