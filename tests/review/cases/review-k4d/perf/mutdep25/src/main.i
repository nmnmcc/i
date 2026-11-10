S(N) { S(f: () -> Int ! N, m: (N) -> Nil) }
use(N) = (s ! S(N)) {
  S(f, ..) = s
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
  _ -> m0(k - 1)
} }
