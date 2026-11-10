f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "a",True) -> 0
("b",_) -> 1
("ab",True) ? False -> 2
} }
