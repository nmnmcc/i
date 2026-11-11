f = () { case "a" <> "b" { "ab" -> 42 _ -> 7 } }


main = () { assert(f() == 42) Nil }
