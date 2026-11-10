f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
(_: String - "ab" - "b",_) -> 0
(_: "a" | "b",_) ? False -> 1
} }
