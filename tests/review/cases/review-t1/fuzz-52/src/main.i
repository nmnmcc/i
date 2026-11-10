f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("b",_) -> 0
("b",_) -> 1
("a" <> _,False) -> 2
(_: String - "ab" - "b",False) -> 3
(_: "a" | "b",True) -> 4
("a",_) -> 5
(_: String - "ab" - "b",False) -> 6
("b",True) -> 7
(_: String - "ab" - "b",False) -> 8
} }
