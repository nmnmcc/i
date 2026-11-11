// syntax: syn1
f = (v) {
   v |> {
    x -> 1
    _ -> 1
    __unused -> 1
    x: Int -> 1
    True | _: Int | String -> 1
    x: { (Int, Int) } -> 1
    x: (Int) -> Int -> 1
    a = b = Some(c) -> 1
    1 | -1 | 1.5 | -.1.5 | "s" -> 1
    "pre" <> rest -> 1
    "pre" <> _ -> 1
    C -> 1
    C() -> 1
    C(..) -> 1
    C(a, b) -> 1
    C(a, ..) -> 1
    C(
      a
      ..
    ) -> 1
    m.C(name = n, ..) -> 1
    () | (a) | (a, b,) -> 1
    [] | [a] | [a, ..] | [a, ..rest] | [..] | [.._] | [a, b,] -> 1
    <<a, b:u8, c:bits(n), "x":utf8, -1:i8, rest:bytes>> -> 1
    <<x = y:u(16)>> -> 1
    Some(x) ? x > 1 -> 1
    Error(_: ParseError) -> 1
    _ -> 2
  }
}
