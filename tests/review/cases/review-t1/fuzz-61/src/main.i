f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,True) -> 0
(_: String - "ab" - "b",True) ? False -> 1
("a" <> _,True) ? False -> 2
} }
