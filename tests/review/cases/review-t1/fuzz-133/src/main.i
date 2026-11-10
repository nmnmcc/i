f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",True) -> 0
("a" <> _,False) -> 1
} }
