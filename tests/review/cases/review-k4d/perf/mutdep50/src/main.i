S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f = f, ..) = s
  f()
}
Dep { Dep(v: Int) }
get = (d ! Dep) { d.v }
m0 = (k: Int) { k |> {
  0 -> get()
  _ -> m1(k - 1)
} }
m1 = (k: Int) { k |> {
  0 -> get()
  _ -> m2(k - 1)
} }
m2 = (k: Int) { k |> {
  0 -> get()
  _ -> m3(k - 1)
} }
m3 = (k: Int) { k |> {
  0 -> get()
  _ -> m4(k - 1)
} }
m4 = (k: Int) { k |> {
  0 -> get()
  _ -> m5(k - 1)
} }
m5 = (k: Int) { k |> {
  0 -> get()
  _ -> m6(k - 1)
} }
m6 = (k: Int) { k |> {
  0 -> get()
  _ -> m7(k - 1)
} }
m7 = (k: Int) { k |> {
  0 -> get()
  _ -> m8(k - 1)
} }
m8 = (k: Int) { k |> {
  0 -> get()
  _ -> m9(k - 1)
} }
m9 = (k: Int) { k |> {
  0 -> get()
  _ -> m10(k - 1)
} }
m10 = (k: Int) { k |> {
  0 -> get()
  _ -> m11(k - 1)
} }
m11 = (k: Int) { k |> {
  0 -> get()
  _ -> m12(k - 1)
} }
m12 = (k: Int) { k |> {
  0 -> get()
  _ -> m13(k - 1)
} }
m13 = (k: Int) { k |> {
  0 -> get()
  _ -> m14(k - 1)
} }
m14 = (k: Int) { k |> {
  0 -> get()
  _ -> m15(k - 1)
} }
m15 = (k: Int) { k |> {
  0 -> get()
  _ -> m16(k - 1)
} }
m16 = (k: Int) { k |> {
  0 -> get()
  _ -> m17(k - 1)
} }
m17 = (k: Int) { k |> {
  0 -> get()
  _ -> m18(k - 1)
} }
m18 = (k: Int) { k |> {
  0 -> get()
  _ -> m19(k - 1)
} }
m19 = (k: Int) { k |> {
  0 -> get()
  _ -> m20(k - 1)
} }
m20 = (k: Int) { k |> {
  0 -> get()
  _ -> m21(k - 1)
} }
m21 = (k: Int) { k |> {
  0 -> get()
  _ -> m22(k - 1)
} }
m22 = (k: Int) { k |> {
  0 -> get()
  _ -> m23(k - 1)
} }
m23 = (k: Int) { k |> {
  0 -> get()
  _ -> m24(k - 1)
} }
m24 = (k: Int) { k |> {
  0 -> get()
  _ -> m25(k - 1)
} }
m25 = (k: Int) { k |> {
  0 -> get()
  _ -> m26(k - 1)
} }
m26 = (k: Int) { k |> {
  0 -> get()
  _ -> m27(k - 1)
} }
m27 = (k: Int) { k |> {
  0 -> get()
  _ -> m28(k - 1)
} }
m28 = (k: Int) { k |> {
  0 -> get()
  _ -> m29(k - 1)
} }
m29 = (k: Int) { k |> {
  0 -> get()
  _ -> m30(k - 1)
} }
m30 = (k: Int) { k |> {
  0 -> get()
  _ -> m31(k - 1)
} }
m31 = (k: Int) { k |> {
  0 -> get()
  _ -> m32(k - 1)
} }
m32 = (k: Int) { k |> {
  0 -> get()
  _ -> m33(k - 1)
} }
m33 = (k: Int) { k |> {
  0 -> get()
  _ -> m34(k - 1)
} }
m34 = (k: Int) { k |> {
  0 -> get()
  _ -> m35(k - 1)
} }
m35 = (k: Int) { k |> {
  0 -> get()
  _ -> m36(k - 1)
} }
m36 = (k: Int) { k |> {
  0 -> get()
  _ -> m37(k - 1)
} }
m37 = (k: Int) { k |> {
  0 -> get()
  _ -> m38(k - 1)
} }
m38 = (k: Int) { k |> {
  0 -> get()
  _ -> m39(k - 1)
} }
m39 = (k: Int) { k |> {
  0 -> get()
  _ -> m40(k - 1)
} }
m40 = (k: Int) { k |> {
  0 -> get()
  _ -> m41(k - 1)
} }
m41 = (k: Int) { k |> {
  0 -> get()
  _ -> m42(k - 1)
} }
m42 = (k: Int) { k |> {
  0 -> get()
  _ -> m43(k - 1)
} }
m43 = (k: Int) { k |> {
  0 -> get()
  _ -> m44(k - 1)
} }
m44 = (k: Int) { k |> {
  0 -> get()
  _ -> m45(k - 1)
} }
m45 = (k: Int) { k |> {
  0 -> get()
  _ -> m46(k - 1)
} }
m46 = (k: Int) { k |> {
  0 -> get()
  _ -> m47(k - 1)
} }
m47 = (k: Int) { k |> {
  0 -> get()
  _ -> m48(k - 1)
} }
m48 = (k: Int) { k |> {
  0 -> get()
  _ -> m49(k - 1)
} }
m49 = (k: Int) { k |> {
  0 -> get()
  _ -> m0(k - 1)
} }
