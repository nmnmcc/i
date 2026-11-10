f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",True) -> 0
(_,False) -> 1
("b",True) ? False -> 2
} }
