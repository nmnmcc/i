f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,True) ? False -> 0
(_: "a" | "b",_) -> 1
("a" <> _,True) ? False -> 2
(_: String - "ab" - "b",_) -> 3
} }
