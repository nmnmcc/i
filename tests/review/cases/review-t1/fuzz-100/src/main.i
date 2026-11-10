f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: "a" | "b",True) -> 0
(_: String - "a",True) -> 1
("b",_) -> 2
("ab",False) ? False -> 3
(_: String - "ab" - "b",_) ? False -> 4
(_: "a" | "b",True) -> 5
(_: String - "a",True) -> 6
} }
