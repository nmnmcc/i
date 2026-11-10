f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",_) ? False -> 0
("a" <> _,True) -> 1
} }
