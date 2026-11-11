f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("ab",False) -> 0
(_: "a" | "b",False) ? False -> 1
("a" <> _,True) -> 2
(_: String - "ab" - "b",True) -> 3
(_,False) -> 4
("b",False) -> 5
(_: "a" | "b",_) -> 6
(_: String - "a",True) -> 7
} }

main = () {
assert(f(("a",True)) == 2)
assert(f(("a",False)) == 4)
assert(f(("ab",True)) == 2)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 6)
assert(f(("b",False)) == 4)
assert(f(("c",True)) == 3)
assert(f(("c",False)) == 4)
assert(f(("",True)) == 3)
assert(f(("",False)) == 4)
Nil
}
