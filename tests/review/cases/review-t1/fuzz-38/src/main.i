f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",_) -> 0
("b",_) -> 1
("a",False) -> 2
(_: String - "ab" - "b",False) -> 3
("ab",True) ? False -> 4
("a" <> _,False) -> 5
} }
