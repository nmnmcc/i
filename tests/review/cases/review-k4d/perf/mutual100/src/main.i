S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
  f()
}
Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
m0 = (k: Int) { k |> {
  0 -> use()
  _ -> m1(k - 1)
} }
m1 = (k: Int) { k |> {
  0 -> use()
  _ -> m2(k - 1)
} }
m2 = (k: Int) { k |> {
  0 -> use()
  _ -> m3(k - 1)
} }
m3 = (k: Int) { k |> {
  0 -> use()
  _ -> m4(k - 1)
} }
m4 = (k: Int) { k |> {
  0 -> use()
  _ -> m5(k - 1)
} }
m5 = (k: Int) { k |> {
  0 -> use()
  _ -> m6(k - 1)
} }
m6 = (k: Int) { k |> {
  0 -> use()
  _ -> m7(k - 1)
} }
m7 = (k: Int) { k |> {
  0 -> use()
  _ -> m8(k - 1)
} }
m8 = (k: Int) { k |> {
  0 -> use()
  _ -> m9(k - 1)
} }
m9 = (k: Int) { k |> {
  0 -> use()
  _ -> m10(k - 1)
} }
m10 = (k: Int) { k |> {
  0 -> use()
  _ -> m11(k - 1)
} }
m11 = (k: Int) { k |> {
  0 -> use()
  _ -> m12(k - 1)
} }
m12 = (k: Int) { k |> {
  0 -> use()
  _ -> m13(k - 1)
} }
m13 = (k: Int) { k |> {
  0 -> use()
  _ -> m14(k - 1)
} }
m14 = (k: Int) { k |> {
  0 -> use()
  _ -> m15(k - 1)
} }
m15 = (k: Int) { k |> {
  0 -> use()
  _ -> m16(k - 1)
} }
m16 = (k: Int) { k |> {
  0 -> use()
  _ -> m17(k - 1)
} }
m17 = (k: Int) { k |> {
  0 -> use()
  _ -> m18(k - 1)
} }
m18 = (k: Int) { k |> {
  0 -> use()
  _ -> m19(k - 1)
} }
m19 = (k: Int) { k |> {
  0 -> use()
  _ -> m20(k - 1)
} }
m20 = (k: Int) { k |> {
  0 -> use()
  _ -> m21(k - 1)
} }
m21 = (k: Int) { k |> {
  0 -> use()
  _ -> m22(k - 1)
} }
m22 = (k: Int) { k |> {
  0 -> use()
  _ -> m23(k - 1)
} }
m23 = (k: Int) { k |> {
  0 -> use()
  _ -> m24(k - 1)
} }
m24 = (k: Int) { k |> {
  0 -> use()
  _ -> m25(k - 1)
} }
m25 = (k: Int) { k |> {
  0 -> use()
  _ -> m26(k - 1)
} }
m26 = (k: Int) { k |> {
  0 -> use()
  _ -> m27(k - 1)
} }
m27 = (k: Int) { k |> {
  0 -> use()
  _ -> m28(k - 1)
} }
m28 = (k: Int) { k |> {
  0 -> use()
  _ -> m29(k - 1)
} }
m29 = (k: Int) { k |> {
  0 -> use()
  _ -> m30(k - 1)
} }
m30 = (k: Int) { k |> {
  0 -> use()
  _ -> m31(k - 1)
} }
m31 = (k: Int) { k |> {
  0 -> use()
  _ -> m32(k - 1)
} }
m32 = (k: Int) { k |> {
  0 -> use()
  _ -> m33(k - 1)
} }
m33 = (k: Int) { k |> {
  0 -> use()
  _ -> m34(k - 1)
} }
m34 = (k: Int) { k |> {
  0 -> use()
  _ -> m35(k - 1)
} }
m35 = (k: Int) { k |> {
  0 -> use()
  _ -> m36(k - 1)
} }
m36 = (k: Int) { k |> {
  0 -> use()
  _ -> m37(k - 1)
} }
m37 = (k: Int) { k |> {
  0 -> use()
  _ -> m38(k - 1)
} }
m38 = (k: Int) { k |> {
  0 -> use()
  _ -> m39(k - 1)
} }
m39 = (k: Int) { k |> {
  0 -> use()
  _ -> m40(k - 1)
} }
m40 = (k: Int) { k |> {
  0 -> use()
  _ -> m41(k - 1)
} }
m41 = (k: Int) { k |> {
  0 -> use()
  _ -> m42(k - 1)
} }
m42 = (k: Int) { k |> {
  0 -> use()
  _ -> m43(k - 1)
} }
m43 = (k: Int) { k |> {
  0 -> use()
  _ -> m44(k - 1)
} }
m44 = (k: Int) { k |> {
  0 -> use()
  _ -> m45(k - 1)
} }
m45 = (k: Int) { k |> {
  0 -> use()
  _ -> m46(k - 1)
} }
m46 = (k: Int) { k |> {
  0 -> use()
  _ -> m47(k - 1)
} }
m47 = (k: Int) { k |> {
  0 -> use()
  _ -> m48(k - 1)
} }
m48 = (k: Int) { k |> {
  0 -> use()
  _ -> m49(k - 1)
} }
m49 = (k: Int) { k |> {
  0 -> use()
  _ -> m50(k - 1)
} }
m50 = (k: Int) { k |> {
  0 -> use()
  _ -> m51(k - 1)
} }
m51 = (k: Int) { k |> {
  0 -> use()
  _ -> m52(k - 1)
} }
m52 = (k: Int) { k |> {
  0 -> use()
  _ -> m53(k - 1)
} }
m53 = (k: Int) { k |> {
  0 -> use()
  _ -> m54(k - 1)
} }
m54 = (k: Int) { k |> {
  0 -> use()
  _ -> m55(k - 1)
} }
m55 = (k: Int) { k |> {
  0 -> use()
  _ -> m56(k - 1)
} }
m56 = (k: Int) { k |> {
  0 -> use()
  _ -> m57(k - 1)
} }
m57 = (k: Int) { k |> {
  0 -> use()
  _ -> m58(k - 1)
} }
m58 = (k: Int) { k |> {
  0 -> use()
  _ -> m59(k - 1)
} }
m59 = (k: Int) { k |> {
  0 -> use()
  _ -> m60(k - 1)
} }
m60 = (k: Int) { k |> {
  0 -> use()
  _ -> m61(k - 1)
} }
m61 = (k: Int) { k |> {
  0 -> use()
  _ -> m62(k - 1)
} }
m62 = (k: Int) { k |> {
  0 -> use()
  _ -> m63(k - 1)
} }
m63 = (k: Int) { k |> {
  0 -> use()
  _ -> m64(k - 1)
} }
m64 = (k: Int) { k |> {
  0 -> use()
  _ -> m65(k - 1)
} }
m65 = (k: Int) { k |> {
  0 -> use()
  _ -> m66(k - 1)
} }
m66 = (k: Int) { k |> {
  0 -> use()
  _ -> m67(k - 1)
} }
m67 = (k: Int) { k |> {
  0 -> use()
  _ -> m68(k - 1)
} }
m68 = (k: Int) { k |> {
  0 -> use()
  _ -> m69(k - 1)
} }
m69 = (k: Int) { k |> {
  0 -> use()
  _ -> m70(k - 1)
} }
m70 = (k: Int) { k |> {
  0 -> use()
  _ -> m71(k - 1)
} }
m71 = (k: Int) { k |> {
  0 -> use()
  _ -> m72(k - 1)
} }
m72 = (k: Int) { k |> {
  0 -> use()
  _ -> m73(k - 1)
} }
m73 = (k: Int) { k |> {
  0 -> use()
  _ -> m74(k - 1)
} }
m74 = (k: Int) { k |> {
  0 -> use()
  _ -> m75(k - 1)
} }
m75 = (k: Int) { k |> {
  0 -> use()
  _ -> m76(k - 1)
} }
m76 = (k: Int) { k |> {
  0 -> use()
  _ -> m77(k - 1)
} }
m77 = (k: Int) { k |> {
  0 -> use()
  _ -> m78(k - 1)
} }
m78 = (k: Int) { k |> {
  0 -> use()
  _ -> m79(k - 1)
} }
m79 = (k: Int) { k |> {
  0 -> use()
  _ -> m80(k - 1)
} }
m80 = (k: Int) { k |> {
  0 -> use()
  _ -> m81(k - 1)
} }
m81 = (k: Int) { k |> {
  0 -> use()
  _ -> m82(k - 1)
} }
m82 = (k: Int) { k |> {
  0 -> use()
  _ -> m83(k - 1)
} }
m83 = (k: Int) { k |> {
  0 -> use()
  _ -> m84(k - 1)
} }
m84 = (k: Int) { k |> {
  0 -> use()
  _ -> m85(k - 1)
} }
m85 = (k: Int) { k |> {
  0 -> use()
  _ -> m86(k - 1)
} }
m86 = (k: Int) { k |> {
  0 -> use()
  _ -> m87(k - 1)
} }
m87 = (k: Int) { k |> {
  0 -> use()
  _ -> m88(k - 1)
} }
m88 = (k: Int) { k |> {
  0 -> use()
  _ -> m89(k - 1)
} }
m89 = (k: Int) { k |> {
  0 -> use()
  _ -> m90(k - 1)
} }
m90 = (k: Int) { k |> {
  0 -> use()
  _ -> m91(k - 1)
} }
m91 = (k: Int) { k |> {
  0 -> use()
  _ -> m92(k - 1)
} }
m92 = (k: Int) { k |> {
  0 -> use()
  _ -> m93(k - 1)
} }
m93 = (k: Int) { k |> {
  0 -> use()
  _ -> m94(k - 1)
} }
m94 = (k: Int) { k |> {
  0 -> use()
  _ -> m95(k - 1)
} }
m95 = (k: Int) { k |> {
  0 -> use()
  _ -> m96(k - 1)
} }
m96 = (k: Int) { k |> {
  0 -> use()
  _ -> m97(k - 1)
} }
m97 = (k: Int) { k |> {
  0 -> use()
  _ -> m98(k - 1)
} }
m98 = (k: Int) { k |> {
  0 -> use()
  _ -> m99(k - 1)
} }
m99 = (k: Int) { k |> {
  0 -> use()
  _ -> m0(k - 1)
} }
