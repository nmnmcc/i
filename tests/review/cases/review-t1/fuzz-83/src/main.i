f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("b",_) -> 0
(_,False) -> 1
(_: "a" | "b",False) -> 2
(_: String - "a",False) -> 3
(_: "a" | "b",_) -> 4
} }
