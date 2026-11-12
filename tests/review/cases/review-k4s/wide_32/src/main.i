S0(N) = (() <- N -> Nil, (N) -> Nil)
u0 = <N>() <- (s: S0(N)) {
	S0(f, _) = s
	f()
}
S1(N) = (() <- N -> Nil, (N) -> Nil)
u1 = <N>() <- (s: S1(N)) {
	S1(f, _) = s
	f()
}
S2(N) = (() <- N -> Nil, (N) -> Nil)
u2 = <N>() <- (s: S2(N)) {
	S2(f, _) = s
	f()
}
S3(N) = (() <- N -> Nil, (N) -> Nil)
u3 = <N>() <- (s: S3(N)) {
	S3(f, _) = s
	f()
}
S4(N) = (() <- N -> Nil, (N) -> Nil)
u4 = <N>() <- (s: S4(N)) {
	S4(f, _) = s
	f()
}
S5(N) = (() <- N -> Nil, (N) -> Nil)
u5 = <N>() <- (s: S5(N)) {
	S5(f, _) = s
	f()
}
S6(N) = (() <- N -> Nil, (N) -> Nil)
u6 = <N>() <- (s: S6(N)) {
	S6(f, _) = s
	f()
}
S7(N) = (() <- N -> Nil, (N) -> Nil)
u7 = <N>() <- (s: S7(N)) {
	S7(f, _) = s
	f()
}
S8(N) = (() <- N -> Nil, (N) -> Nil)
u8 = <N>() <- (s: S8(N)) {
	S8(f, _) = s
	f()
}
S9(N) = (() <- N -> Nil, (N) -> Nil)
u9 = <N>() <- (s: S9(N)) {
	S9(f, _) = s
	f()
}
S10(N) = (() <- N -> Nil, (N) -> Nil)
u10 = <N>() <- (s: S10(N)) {
	S10(f, _) = s
	f()
}
S11(N) = (() <- N -> Nil, (N) -> Nil)
u11 = <N>() <- (s: S11(N)) {
	S11(f, _) = s
	f()
}
S12(N) = (() <- N -> Nil, (N) -> Nil)
u12 = <N>() <- (s: S12(N)) {
	S12(f, _) = s
	f()
}
S13(N) = (() <- N -> Nil, (N) -> Nil)
u13 = <N>() <- (s: S13(N)) {
	S13(f, _) = s
	f()
}
S14(N) = (() <- N -> Nil, (N) -> Nil)
u14 = <N>() <- (s: S14(N)) {
	S14(f, _) = s
	f()
}
S15(N) = (() <- N -> Nil, (N) -> Nil)
u15 = <N>() <- (s: S15(N)) {
	S15(f, _) = s
	f()
}
S16(N) = (() <- N -> Nil, (N) -> Nil)
u16 = <N>() <- (s: S16(N)) {
	S16(f, _) = s
	f()
}
S17(N) = (() <- N -> Nil, (N) -> Nil)
u17 = <N>() <- (s: S17(N)) {
	S17(f, _) = s
	f()
}
S18(N) = (() <- N -> Nil, (N) -> Nil)
u18 = <N>() <- (s: S18(N)) {
	S18(f, _) = s
	f()
}
S19(N) = (() <- N -> Nil, (N) -> Nil)
u19 = <N>() <- (s: S19(N)) {
	S19(f, _) = s
	f()
}
S20(N) = (() <- N -> Nil, (N) -> Nil)
u20 = <N>() <- (s: S20(N)) {
	S20(f, _) = s
	f()
}
S21(N) = (() <- N -> Nil, (N) -> Nil)
u21 = <N>() <- (s: S21(N)) {
	S21(f, _) = s
	f()
}
S22(N) = (() <- N -> Nil, (N) -> Nil)
u22 = <N>() <- (s: S22(N)) {
	S22(f, _) = s
	f()
}
S23(N) = (() <- N -> Nil, (N) -> Nil)
u23 = <N>() <- (s: S23(N)) {
	S23(f, _) = s
	f()
}
S24(N) = (() <- N -> Nil, (N) -> Nil)
u24 = <N>() <- (s: S24(N)) {
	S24(f, _) = s
	f()
}
S25(N) = (() <- N -> Nil, (N) -> Nil)
u25 = <N>() <- (s: S25(N)) {
	S25(f, _) = s
	f()
}
S26(N) = (() <- N -> Nil, (N) -> Nil)
u26 = <N>() <- (s: S26(N)) {
	S26(f, _) = s
	f()
}
S27(N) = (() <- N -> Nil, (N) -> Nil)
u27 = <N>() <- (s: S27(N)) {
	S27(f, _) = s
	f()
}
S28(N) = (() <- N -> Nil, (N) -> Nil)
u28 = <N>() <- (s: S28(N)) {
	S28(f, _) = s
	f()
}
S29(N) = (() <- N -> Nil, (N) -> Nil)
u29 = <N>() <- (s: S29(N)) {
	S29(f, _) = s
	f()
}
S30(N) = (() <- N -> Nil, (N) -> Nil)
u30 = <N>() <- (s: S30(N)) {
	S30(f, _) = s
	f()
}
S31(N) = (() <- N -> Nil, (N) -> Nil)
u31 = <N>() <- (s: S31(N)) {
	S31(f, _) = s
	f()
}
run = <A, N>(body: () <- N -> A) { body() }
wide = () {
	run(() { u0() })
	run(() { u1() })
	run(() { u2() })
	run(() { u3() })
	run(() { u4() })
	run(() { u5() })
	run(() { u6() })
	run(() { u7() })
	run(() { u8() })
	run(() { u9() })
	run(() { u10() })
	run(() { u11() })
	run(() { u12() })
	run(() { u13() })
	run(() { u14() })
	run(() { u15() })
	run(() { u16() })
	run(() { u17() })
	run(() { u18() })
	run(() { u19() })
	run(() { u20() })
	run(() { u21() })
	run(() { u22() })
	run(() { u23() })
	run(() { u24() })
	run(() { u25() })
	run(() { u26() })
	run(() { u27() })
	run(() { u28() })
	run(() { u29() })
	run(() { u30() })
	run(() { u31() })
}
