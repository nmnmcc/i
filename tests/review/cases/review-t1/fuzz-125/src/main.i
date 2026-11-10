f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",True) -> 0
(_: "a" | "b",False) -> 1
("a" <> _,_) ? False -> 2
("a",False) ? False -> 3
} }
