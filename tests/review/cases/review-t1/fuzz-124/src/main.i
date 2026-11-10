f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",True) -> 0
("b",False) -> 1
(_: String - "a",True) ? False -> 2
(_: "a" | "b",True) -> 3
("ab",_) -> 4
(_: String - "a",False) -> 5
("ab",_) -> 6
("a",_) -> 7
} }
