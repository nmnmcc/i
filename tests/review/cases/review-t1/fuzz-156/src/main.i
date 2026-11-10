f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",_) ? False -> 0
("ab",False) -> 1
("a",_) -> 2
("a" <> _,False) -> 3
("ab",_) -> 4
("a" <> _,_) -> 5
("a" <> _,False) -> 6
} }
