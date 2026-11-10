f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,False) -> 0
(_: String - "a",True) ? False -> 1
} }
