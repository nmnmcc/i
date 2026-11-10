f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",False) -> 0
("a" <> _,_) -> 1
(_: "a" | "b",False) -> 2
(_,False) -> 3
(_,_) ? False -> 4
(_: String - "ab" - "b",False) -> 5
} }
