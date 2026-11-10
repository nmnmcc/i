f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,False) -> 0
("a",True) -> 1
(_: String - "ab" - "b",False) -> 2
(_: String - "a",False) -> 3
(_: "a" | "b",_) -> 4
("a",False) -> 5
(_,False) -> 6
} }
