f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",_) -> 0
(_,True) -> 1
("a",True) -> 2
("a" <> _,_) -> 3
("a" <> _,True) ? False -> 4
} }
