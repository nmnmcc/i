f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a",_) -> 0
(_: String - "a",True) -> 1
(_: "a" | "b",True) -> 2
(_: "a" | "b",False) -> 3
(_,True) -> 4
("a" <> _,False) ? False -> 5
("a",True) -> 6
("a" <> _,_) -> 7
(_,False) ? False -> 8
} }
