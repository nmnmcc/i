f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("b",_) -> 0
(_: String - "ab" - "b",True) -> 1
(_,True) -> 2
("ab",True) -> 3
("a",_) ? False -> 4
("b",False) -> 5
("a",True) ? False -> 6
} }
