S0(A) { S0(A) }
u0 = (s ! S0(Int)) { Nil }
S1(A) { S1(A) }
u1 = (s ! S1(Int)) { Nil }
S2(A) { S2(A) }
u2 = (s ! S2(Int)) { Nil }
S3(A) { S3(A) }
u3 = (s ! S3(Int)) { Nil }
S4(A) { S4(A) }
u4 = (s ! S4(Int)) { Nil }
S5(A) { S5(A) }
u5 = (s ! S5(Int)) { Nil }
S6(A) { S6(A) }
u6 = (s ! S6(Int)) { Nil }
S7(A) { S7(A) }
u7 = (s ! S7(Int)) { Nil }
S8(A) { S8(A) }
u8 = (s ! S8(Int)) { Nil }
S9(A) { S9(A) }
u9 = (s ! S9(Int)) { Nil }
S10(A) { S10(A) }
u10 = (s ! S10(Int)) { Nil }
S11(A) { S11(A) }
u11 = (s ! S11(Int)) { Nil }
S12(A) { S12(A) }
u12 = (s ! S12(Int)) { Nil }
S13(A) { S13(A) }
u13 = (s ! S13(Int)) { Nil }
S14(A) { S14(A) }
u14 = (s ! S14(Int)) { Nil }
S15(A) { S15(A) }
u15 = (s ! S15(Int)) { Nil }
S16(A) { S16(A) }
u16 = (s ! S16(Int)) { Nil }
S17(A) { S17(A) }
u17 = (s ! S17(Int)) { Nil }
S18(A) { S18(A) }
u18 = (s ! S18(Int)) { Nil }
S19(A) { S19(A) }
u19 = (s ! S19(Int)) { Nil }
S20(A) { S20(A) }
u20 = (s ! S20(Int)) { Nil }
S21(A) { S21(A) }
u21 = (s ! S21(Int)) { Nil }
S22(A) { S22(A) }
u22 = (s ! S22(Int)) { Nil }
S23(A) { S23(A) }
u23 = (s ! S23(Int)) { Nil }
S24(A) { S24(A) }
u24 = (s ! S24(Int)) { Nil }
S25(A) { S25(A) }
u25 = (s ! S25(Int)) { Nil }
S26(A) { S26(A) }
u26 = (s ! S26(Int)) { Nil }
S27(A) { S27(A) }
u27 = (s ! S27(Int)) { Nil }
S28(A) { S28(A) }
u28 = (s ! S28(Int)) { Nil }
S29(A) { S29(A) }
u29 = (s ! S29(Int)) { Nil }
S30(A) { S30(A) }
u30 = (s ! S30(Int)) { Nil }
S31(A) { S31(A) }
u31 = (s ! S31(Int)) { Nil }
S32(A) { S32(A) }
u32 = (s ! S32(Int)) { Nil }
S33(A) { S33(A) }
u33 = (s ! S33(Int)) { Nil }
S34(A) { S34(A) }
u34 = (s ! S34(Int)) { Nil }
S35(A) { S35(A) }
u35 = (s ! S35(Int)) { Nil }
S36(A) { S36(A) }
u36 = (s ! S36(Int)) { Nil }
S37(A) { S37(A) }
u37 = (s ! S37(Int)) { Nil }
S38(A) { S38(A) }
u38 = (s ! S38(Int)) { Nil }
S39(A) { S39(A) }
u39 = (s ! S39(Int)) { Nil }
S40(A) { S40(A) }
u40 = (s ! S40(Int)) { Nil }
S41(A) { S41(A) }
u41 = (s ! S41(Int)) { Nil }
S42(A) { S42(A) }
u42 = (s ! S42(Int)) { Nil }
S43(A) { S43(A) }
u43 = (s ! S43(Int)) { Nil }
S44(A) { S44(A) }
u44 = (s ! S44(Int)) { Nil }
S45(A) { S45(A) }
u45 = (s ! S45(Int)) { Nil }
S46(A) { S46(A) }
u46 = (s ! S46(Int)) { Nil }
S47(A) { S47(A) }
u47 = (s ! S47(Int)) { Nil }
S48(A) { S48(A) }
u48 = (s ! S48(Int)) { Nil }
S49(A) { S49(A) }
u49 = (s ! S49(Int)) { Nil }
S50(A) { S50(A) }
u50 = (s ! S50(Int)) { Nil }
S51(A) { S51(A) }
u51 = (s ! S51(Int)) { Nil }
S52(A) { S52(A) }
u52 = (s ! S52(Int)) { Nil }
S53(A) { S53(A) }
u53 = (s ! S53(Int)) { Nil }
S54(A) { S54(A) }
u54 = (s ! S54(Int)) { Nil }
S55(A) { S55(A) }
u55 = (s ! S55(Int)) { Nil }
S56(A) { S56(A) }
u56 = (s ! S56(Int)) { Nil }
S57(A) { S57(A) }
u57 = (s ! S57(Int)) { Nil }
S58(A) { S58(A) }
u58 = (s ! S58(Int)) { Nil }
S59(A) { S59(A) }
u59 = (s ! S59(Int)) { Nil }
S60(A) { S60(A) }
u60 = (s ! S60(Int)) { Nil }
S61(A) { S61(A) }
u61 = (s ! S61(Int)) { Nil }
S62(A) { S62(A) }
u62 = (s ! S62(Int)) { Nil }
S63(A) { S63(A) }
u63 = (s ! S63(Int)) { Nil }
run(A,N) = (body: () -> A ! N) { body() }
wide = () { run(() { u0() }) run(() { u1() }) run(() { u2() }) run(() { u3() }) run(() { u4() }) run(() { u5() }) run(() { u6() }) run(() { u7() }) run(() { u8() }) run(() { u9() }) run(() { u10() }) run(() { u11() }) run(() { u12() }) run(() { u13() }) run(() { u14() }) run(() { u15() }) run(() { u16() }) run(() { u17() }) run(() { u18() }) run(() { u19() }) run(() { u20() }) run(() { u21() }) run(() { u22() }) run(() { u23() }) run(() { u24() }) run(() { u25() }) run(() { u26() }) run(() { u27() }) run(() { u28() }) run(() { u29() }) run(() { u30() }) run(() { u31() }) run(() { u32() }) run(() { u33() }) run(() { u34() }) run(() { u35() }) run(() { u36() }) run(() { u37() }) run(() { u38() }) run(() { u39() }) run(() { u40() }) run(() { u41() }) run(() { u42() }) run(() { u43() }) run(() { u44() }) run(() { u45() }) run(() { u46() }) run(() { u47() }) run(() { u48() }) run(() { u49() }) run(() { u50() }) run(() { u51() }) run(() { u52() }) run(() { u53() }) run(() { u54() }) run(() { u55() }) run(() { u56() }) run(() { u57() }) run(() { u58() }) run(() { u59() }) run(() { u60() }) run(() { u61() }) run(() { u62() }) run(() { u63() }) }

