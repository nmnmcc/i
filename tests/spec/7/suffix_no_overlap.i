// spec: 7.1
// expect: pass
main = () {
    assert("aba" |> { "ab" <> _ <> "ba" -> False  _ -> True })
    assert("abba" |> { "ab" <> r <> "ba" -> r == ""  _ -> False })
    Nil
}
