f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,True) -> 0
(_: String - "ab" - "b",False) ? False -> 1
("ab",_) -> 2
(_: String - "a",_) -> 3
(_: "a" | "b",True) -> 4
("a",True) -> 5
} }
