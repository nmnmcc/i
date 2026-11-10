f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("ab",False) -> 0
("a",False) -> 1
("b",True) -> 2
} }
