f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",True) -> 0
(_: String - "ab" - "b",False) ? False -> 1
} }
