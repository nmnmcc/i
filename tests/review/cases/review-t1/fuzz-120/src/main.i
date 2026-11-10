f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("b",_) -> 0
(_: String - "a",False) -> 1
(_: String - "a",_) -> 2
("b",_) -> 3
(_,True) -> 4
} }
