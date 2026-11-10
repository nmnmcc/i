f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_,False) ? False -> 0
("ab",_) ? False -> 1
("b",True) -> 2
("ab",True) -> 3
("b",False) ? False -> 4
} }
