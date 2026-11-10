f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "a",_) -> 0
(_: String - "ab" - "b",_) ? False -> 1
(_: String - "ab" - "b",True) -> 2
("ab",False) -> 3
("ab",_) -> 4
} }
