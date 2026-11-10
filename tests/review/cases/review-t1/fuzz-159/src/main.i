f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("ab",_) -> 0
(_: String - "a",True) -> 1
(_,True) -> 2
} }
