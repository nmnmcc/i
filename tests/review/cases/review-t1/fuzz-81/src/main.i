f = (x: ("a" | "ab" | "b" | "c" | "", Bool)) { x |> {
("a" <> _,False) -> 0
("a" <> _,False) -> 1
(_: String - "ab" - "b",True) -> 2
("a" <> _,False) -> 3
(_,_) -> 4
(_: String - "ab" - "b",_) -> 5
(_: "a" | "b",_) -> 6
("ab",False) -> 7
("ab",True) -> 8
} }

main = () {
assert(f(("a",True)) == 2)
assert(f(("a",False)) == 0)
assert(f(("ab",True)) == 4)
assert(f(("ab",False)) == 0)
assert(f(("b",True)) == 4)
assert(f(("b",False)) == 4)
assert(f(("c",True)) == 2)
assert(f(("c",False)) == 4)
assert(f(("",True)) == 2)
assert(f(("",False)) == 4)
Nil
}
