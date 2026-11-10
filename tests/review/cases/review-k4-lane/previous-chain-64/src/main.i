S(N) {
	S(() -> Nil ! N, (N) -> Nil)
}

use(N) = (s ! S(N)) {
	S(f, _) = s
	f()
}

run(N) = (body: () -> Nil ! N) { body() }

forward_0 = () { run(() { use() }) }

forward_1 = () { run(() { forward_0() }) }

forward_2 = () { run(() { forward_1() }) }

forward_3 = () { run(() { forward_2() }) }

forward_4 = () { run(() { forward_3() }) }

forward_5 = () { run(() { forward_4() }) }

forward_6 = () { run(() { forward_5() }) }

forward_7 = () { run(() { forward_6() }) }

forward_8 = () { run(() { forward_7() }) }

forward_9 = () { run(() { forward_8() }) }

forward_10 = () { run(() { forward_9() }) }

forward_11 = () { run(() { forward_10() }) }

forward_12 = () { run(() { forward_11() }) }

forward_13 = () { run(() { forward_12() }) }

forward_14 = () { run(() { forward_13() }) }

forward_15 = () { run(() { forward_14() }) }

forward_16 = () { run(() { forward_15() }) }

forward_17 = () { run(() { forward_16() }) }

forward_18 = () { run(() { forward_17() }) }

forward_19 = () { run(() { forward_18() }) }

forward_20 = () { run(() { forward_19() }) }

forward_21 = () { run(() { forward_20() }) }

forward_22 = () { run(() { forward_21() }) }

forward_23 = () { run(() { forward_22() }) }

forward_24 = () { run(() { forward_23() }) }

forward_25 = () { run(() { forward_24() }) }

forward_26 = () { run(() { forward_25() }) }

forward_27 = () { run(() { forward_26() }) }

forward_28 = () { run(() { forward_27() }) }

forward_29 = () { run(() { forward_28() }) }

forward_30 = () { run(() { forward_29() }) }

forward_31 = () { run(() { forward_30() }) }

forward_32 = () { run(() { forward_31() }) }

forward_33 = () { run(() { forward_32() }) }

forward_34 = () { run(() { forward_33() }) }

forward_35 = () { run(() { forward_34() }) }

forward_36 = () { run(() { forward_35() }) }

forward_37 = () { run(() { forward_36() }) }

forward_38 = () { run(() { forward_37() }) }

forward_39 = () { run(() { forward_38() }) }

forward_40 = () { run(() { forward_39() }) }

forward_41 = () { run(() { forward_40() }) }

forward_42 = () { run(() { forward_41() }) }

forward_43 = () { run(() { forward_42() }) }

forward_44 = () { run(() { forward_43() }) }

forward_45 = () { run(() { forward_44() }) }

forward_46 = () { run(() { forward_45() }) }

forward_47 = () { run(() { forward_46() }) }

forward_48 = () { run(() { forward_47() }) }

forward_49 = () { run(() { forward_48() }) }

forward_50 = () { run(() { forward_49() }) }

forward_51 = () { run(() { forward_50() }) }

forward_52 = () { run(() { forward_51() }) }

forward_53 = () { run(() { forward_52() }) }

forward_54 = () { run(() { forward_53() }) }

forward_55 = () { run(() { forward_54() }) }

forward_56 = () { run(() { forward_55() }) }

forward_57 = () { run(() { forward_56() }) }

forward_58 = () { run(() { forward_57() }) }

forward_59 = () { run(() { forward_58() }) }

forward_60 = () { run(() { forward_59() }) }

forward_61 = () { run(() { forward_60() }) }

forward_62 = () { run(() { forward_61() }) }

forward_63 = () { run(() { forward_62() }) }

@main
main = () { forward_63() }
